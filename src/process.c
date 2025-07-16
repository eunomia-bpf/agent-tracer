// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
/* Copyright (c) 2020 Facebook */
#include <argp.h>
#include <signal.h>
#include <stdio.h>
#include <time.h>
#include <sys/resource.h>
#include <bpf/libbpf.h>
#include <dirent.h>
#include <string.h>
#include <stdlib.h>
#include <errno.h>
#include "process.h"
#include "process.skel.h"
#include "process_utils.h"

#define MAX_COMMAND_LIST 256
#define FILE_DEDUP_WINDOW_NS 60000000000ULL  // 60 seconds in nanoseconds
#define MAX_FILE_HASHES 1024

// Simple hash table for FILE_OPEN deduplication
struct file_hash_entry {
    uint64_t hash;
    uint64_t timestamp_ns;
    uint32_t count;
    pid_t pid;
    char comm[TASK_COMM_LEN];
    char filepath[MAX_FILENAME_LEN];
    int flags;
};

static struct file_hash_entry file_hashes[MAX_FILE_HASHES];
static int hash_count = 0;

static struct env {
	bool verbose;
	long min_duration_ms;
	char *command_list[MAX_COMMAND_LIST];
	int command_count;
	enum filter_mode filter_mode;
	pid_t pid;
} env = {
	.verbose = false,
	.min_duration_ms = 0,
	.command_count = 0,
	.filter_mode = FILTER_MODE_PROC,
	.pid = 0
};

const char *argp_program_version = "process-tracer 1.0";
const char *argp_program_bug_address = "<bpf@vger.kernel.org>";
const char argp_program_doc[] =
"BPF process tracer with 3-level filtering.\n"
"\n"
"It traces process start and exits with configurable filtering levels.\n"
"Shows associated information (filename, process duration, PID and PPID, etc).\n"
"\n"
"USAGE: ./process [-d <min-duration-ms>] [-c <command1,command2,...>] [-p <pid>] [-m <mode>] [-v]\n"
"\n"
"FILTER MODES:\n"
"  0 (all):    Trace all processes and all read/write operations\n"
"  1 (proc):   Trace all processes but only read/write for tracked PIDs\n"
"  2 (filter): Only trace processes matching filters and their read/write (default)\n"
"\n"
"EXAMPLES:\n"
"  ./process -m 0                   # Trace everything\n"
"  ./process -m 1                   # Trace all processes, selective read/write\n"
"  ./process -c \"claude,python\"    # Trace only claude/python processes\n"
"  ./process -c \"ssh\" -d 1000     # Trace ssh processes lasting > 1 second\n"
"  ./process -p 1234                # Trace only PID 1234\n";

static const struct argp_option opts[] = {
	{ "verbose", 'v', NULL, 0, "Verbose debug output" },
	{ "duration", 'd', "DURATION-MS", 0, "Minimum process duration (ms) to report" },
	{ "commands", 'c', "COMMAND-LIST", 0, "Comma-separated list of commands to trace (e.g., \"claude,python\")" },
	{ "pid", 'p', "PID", 0, "Trace this PID only" },
	{ "mode", 'm', "FILTER-MODE", 0, "Filter mode: 0=all, 1=proc, 2=filter (default=2)" },
	{ "all", 'a', NULL, 0, "Deprecated: use -m 0 instead" },
	{},
};

static error_t parse_arg(int key, char *arg, struct argp_state *state)
{
	char *token;
	char *saveptr;
	
	switch (key) {
	case 'v':
		env.verbose = true;
		break;
	case 'd':
		errno = 0;
		env.min_duration_ms = strtol(arg, NULL, 10);
		if (errno || env.min_duration_ms <= 0) {
			fprintf(stderr, "Invalid duration: %s\n", arg);
			argp_usage(state);
		}
		break;
	case 'p':
		errno = 0;
		env.pid = (pid_t)strtol(arg, NULL, 10);
		if (errno || env.pid <= 0) {
			fprintf(stderr, "Invalid PID: %s\n", arg);
			argp_usage(state);
		}
		env.filter_mode = FILTER_MODE_FILTER;
		break;
	case 'a':
		env.filter_mode = FILTER_MODE_ALL;
		break;
	case 'm':
		errno = 0;
		int mode = strtol(arg, NULL, 10);
		if (errno || mode < 0 || mode > 2) {
			fprintf(stderr, "Invalid filter mode: %s (must be 0, 1, or 2)\n", arg);
			argp_usage(state);
		}
		env.filter_mode = (enum filter_mode)mode;
		break;
	case 'c':
		env.filter_mode = FILTER_MODE_FILTER;
		/* Parse comma-separated command list */
		char *arg_copy = strdup(arg);
		if (!arg_copy) {
			fprintf(stderr, "Memory allocation failed\n");
			return ARGP_ERR_UNKNOWN;
		}
		
		token = strtok_r(arg_copy, ",", &saveptr);
		while (token && env.command_count < MAX_COMMAND_LIST) {
			/* Remove leading/trailing whitespace */
			while (*token == ' ' || *token == '\t') token++;
			char *end = token + strlen(token) - 1;
			while (end > token && (*end == ' ' || *end == '\t')) end--;
			*(end + 1) = '\0';
			
			if (strlen(token) > 0) {
				env.command_list[env.command_count] = strdup(token);
				if (!env.command_list[env.command_count]) {
					fprintf(stderr, "Memory allocation failed\n");
					free(arg_copy);
					return ARGP_ERR_UNKNOWN;
				}
				env.command_count++;
			}
			token = strtok_r(NULL, ",", &saveptr);
		}
		free(arg_copy);
		break;
	case ARGP_KEY_ARG:
		argp_usage(state);
		break;
	default:
		return ARGP_ERR_UNKNOWN;
	}
	return 0;
}

static const struct argp argp = {
	.options = opts,
	.parser = parse_arg,
	.doc = argp_program_doc,
};

static int libbpf_print_fn(enum libbpf_print_level level, const char *format, va_list args)
{
	if (level == LIBBPF_DEBUG && !env.verbose)
		return 0;
	return vfprintf(stderr, format, args);
}

static volatile bool exiting = false;

// Shared function to print FILE_OPEN events
static void print_file_open_event(const struct event *e, uint64_t timestamp_ns, uint32_t count, const char *extra_fields)
{
	printf("{");
	printf("\"timestamp\":%llu,", timestamp_ns);
	printf("\"event\":\"FILE_OPEN\",");
	printf("\"comm\":\"%s\",", e->comm);
	printf("\"pid\":%d,", e->pid);
	printf("\"count\":%u,", count);
	printf("\"filepath\":\"%s\",", e->file_op.filepath);
	printf("\"flags\":%d", e->file_op.flags);
	
	if (extra_fields && strlen(extra_fields) > 0) {
		printf(",%s", extra_fields);
	}
	
	printf("}\n");
}


// Hash function for FILE_OPEN events
static uint64_t hash_file_open(const struct event *e)
{
	uint64_t hash = 5381;
	hash = ((hash << 5) + hash) + e->pid;
	
	// Hash the filepath for FILE_OPEN events
	const char *str = e->file_op.filepath;
	while (*str) {
		hash = ((hash << 5) + hash) + *str++;
	}
	
	return hash;
}

// Get count for FILE_OPEN operations (handles deduplication internally)
static uint32_t get_file_open_count(const struct event *e, uint64_t timestamp_ns)
{
	if (e->type != EVENT_TYPE_FILE_OPERATION || !e->file_op.is_open) {
		return 1;  // Return count of 1 for non-FILE_OPEN operations
	}
	
	uint64_t hash = hash_file_open(e);
	
	// Clean up expired entries first
	for (int i = 0; i < hash_count; i++) {
		if (timestamp_ns - file_hashes[i].timestamp_ns > FILE_DEDUP_WINDOW_NS) {
			// Print aggregated result if count > 1
			if (file_hashes[i].count > 1) {
				if (env.verbose) {
					fprintf(stderr, "DEBUG: Aggregation window expired for FILE_OPEN, count=%u\n", 
						file_hashes[i].count);
				}
				// Create fake event structure for aggregated output
				struct event fake_event = {
					.type = EVENT_TYPE_FILE_OPERATION,
					.pid = file_hashes[i].pid,
					.ppid = 0,
					.exit_code = 0,
					.duration_ns = 0,
					.exit_event = false,
					.file_op = {
						.fd = -1,
						.flags = file_hashes[i].flags,
						.is_open = true
					}
				};
				strncpy(fake_event.comm, file_hashes[i].comm, TASK_COMM_LEN - 1);
				fake_event.comm[TASK_COMM_LEN - 1] = '\0';
				strncpy(fake_event.file_op.filepath, file_hashes[i].filepath, MAX_FILENAME_LEN - 1);
				fake_event.file_op.filepath[MAX_FILENAME_LEN - 1] = '\0';
				print_file_open_event(&fake_event, timestamp_ns, file_hashes[i].count, "\"window_expired\":true");
			}
			
			// Remove expired entry
			file_hashes[i] = file_hashes[hash_count - 1];
			hash_count--;
			i--;
		}
	}
	
	// Check if this hash already exists
	for (int i = 0; i < hash_count; i++) {
		if (file_hashes[i].hash == hash) {
			file_hashes[i].count++;
			file_hashes[i].timestamp_ns = timestamp_ns;
			if (env.verbose) {
				fprintf(stderr, "DEBUG: Aggregating FILE_OPEN for PID %d, count now %u\n", 
					e->pid, file_hashes[i].count);
			}
			return 0;  // Return 0 to indicate this should be skipped (duplicate)
		}
	}
	
	// Add new hash entry if we have space
	if (hash_count < MAX_FILE_HASHES) {
		file_hashes[hash_count].hash = hash;
		file_hashes[hash_count].timestamp_ns = timestamp_ns;
		file_hashes[hash_count].count = 1;
		file_hashes[hash_count].pid = e->pid;
		strncpy(file_hashes[hash_count].comm, e->comm, TASK_COMM_LEN - 1);
		file_hashes[hash_count].comm[TASK_COMM_LEN - 1] = '\0';
		strncpy(file_hashes[hash_count].filepath, e->file_op.filepath, MAX_FILENAME_LEN - 1);
		file_hashes[hash_count].filepath[MAX_FILENAME_LEN - 1] = '\0';
		file_hashes[hash_count].flags = e->file_op.flags;
		hash_count++;
		if (env.verbose) {
			fprintf(stderr, "DEBUG: Created new aggregation entry for FILE_OPEN, PID %d (total entries: %d)\n", 
				e->pid, hash_count);
		}
	} else if (env.verbose) {
		fprintf(stderr, "DEBUG: Max aggregation entries reached (%d), cannot track more\n", MAX_FILE_HASHES);
		// just print the event
		print_file_open_event(e, timestamp_ns, 1, NULL);
	}
	
	return 1;  // Return count of 1 for first occurrence
}

// Flush all pending FILE_OPEN aggregations for a specific PID
static void flush_pid_file_opens(pid_t pid, uint64_t timestamp_ns)
{
	int flushed_count = 0;
	for (int i = 0; i < hash_count; i++) {
		if (file_hashes[i].pid == pid && file_hashes[i].count > 1) {
			if (env.verbose) {
				fprintf(stderr, "DEBUG: Flushing FILE_OPEN aggregation on process exit, PID %d, count=%u\n", 
					pid, file_hashes[i].count);
			}
			// Create fake event structure for aggregated output
			struct event fake_event = {
				.type = EVENT_TYPE_FILE_OPERATION,
				.pid = file_hashes[i].pid,
				.ppid = 0,
				.exit_code = 0,
				.duration_ns = 0,
				.exit_event = false,
				.file_op = {
					.fd = -1,
					.flags = file_hashes[i].flags,
					.is_open = true
				}
			};
			strncpy(fake_event.comm, file_hashes[i].comm, TASK_COMM_LEN - 1);
			fake_event.comm[TASK_COMM_LEN - 1] = '\0';
			strncpy(fake_event.file_op.filepath, file_hashes[i].filepath, MAX_FILENAME_LEN - 1);
			fake_event.file_op.filepath[MAX_FILENAME_LEN - 1] = '\0';
			print_file_open_event(&fake_event, timestamp_ns, file_hashes[i].count, "\"reason\":\"process_exit\"");
			flushed_count++;
		}
	}
	
	// Remove all entries for this PID
	int removed_count = 0;
	for (int i = 0; i < hash_count; i++) {
		if (file_hashes[i].pid == pid) {
			// Remove this entry by moving last entry to this position
			file_hashes[i] = file_hashes[hash_count - 1];
			hash_count--;
			removed_count++;
			i--; // Recheck this position since we moved an entry here
		}
	}
	
	if (env.verbose && removed_count > 0) {
		fprintf(stderr, "DEBUG: Cleared %d FILE_OPEN aggregation entries for PID %d (flushed %d)\n", 
			removed_count, pid, flushed_count);
	}
}

static void sig_handler(int sig)
{
	exiting = true;
}


static int setup_command_filters(struct process_bpf *skel, char **command_list, int command_count)
{
	for (int i = 0; i < command_count && i < MAX_COMMAND_FILTERS; i++) {
		struct command_filter filter = {
		};
		
		strncpy(filter.comm, command_list[i], TASK_COMM_LEN - 1);
		filter.comm[TASK_COMM_LEN - 1] = '\0';
		
		skel->rodata->command_filters[i] = filter;
	}
	
	return 0;
}

/* Populate initial PIDs in the eBPF map from existing processes */
static int populate_initial_pids(struct process_bpf *skel, char **command_list, int command_count, enum filter_mode filter_mode, pid_t **tracked_pids_out)
{
	DIR *proc_dir;
	struct dirent *entry;
	pid_t pid, ppid;
	char comm[TASK_COMM_LEN];
	int tracked_count = 0;
	static pid_t tracked_pids_array[MAX_TRACKED_PIDS];
	*tracked_pids_out = tracked_pids_array;
	
	proc_dir = opendir("/proc");
	if (!proc_dir) {
		fprintf(stderr, "Failed to open /proc directory\n");
		return -1;
	}
	
	while ((entry = readdir(proc_dir)) != NULL) {
		/* Skip non-numeric entries */
		if (strspn(entry->d_name, "0123456789") != strlen(entry->d_name))
			continue;
		
		pid = (pid_t)strtol(entry->d_name, NULL, 10);
		if (pid <= 0)
			continue;
		
		/* Read process command */
		if (read_proc_comm(pid, comm, sizeof(comm)) != 0)
			continue;
		
		/* Read parent PID */
		if (read_proc_ppid(pid, &ppid) != 0)
			continue;
		
		bool should_track = (filter_mode == FILTER_MODE_ALL);
		
		/* If using filter mode, check if this process matches any configured filter */
		if (filter_mode == FILTER_MODE_FILTER) {
			should_track = false;
			
			/* Check if PID matches if -p option was specified */
			if (env.pid > 0) {
				if (pid == env.pid) {
					should_track = true;
				}
			}
			/* Otherwise check command filters */
			else if (command_list && command_count > 0) {
				for (int i = 0; i < command_count; i++) {
					if (command_matches_filter(comm, command_list[i])) {
						should_track = true;
						break;
					}
				}
			}
		}
		
		if (should_track) {
			/* Add to tracked PIDs in eBPF map */
			struct pid_info pid_info = {
				.pid = pid,
				.ppid = ppid,
				.is_tracked = true
			};
			
			int err = bpf_map__update_elem(skel->maps.tracked_pids, &pid, sizeof(pid),
			                               &pid_info, sizeof(pid_info), BPF_ANY);
			if (err && filter_mode != FILTER_MODE_ALL) {  /* Don't spam errors when tracing all processes */
				fprintf(stderr, "Failed to add PID %d to tracked list: %d\n", pid, err);
			}
			if (!err) {
				if (tracked_count < MAX_TRACKED_PIDS) {
					tracked_pids_array[tracked_count] = pid;
				}
				tracked_count++;
			}
		}
	}
	
	closedir(proc_dir);
	return tracked_count;
}

static int handle_event(void *ctx, void *data, size_t data_sz)
{
	const struct event *e = data;
	
	// Use kernel timestamp from the event instead of generating our own
	uint64_t timestamp_ns = e->timestamp_ns;

	switch (e->type) {
		case EVENT_TYPE_PROCESS:
			// For process events, always report immediately
			printf("{");
			printf("\"timestamp\":%llu,", timestamp_ns);
			printf("\"event\":\"%s\",", e->exit_event ? "EXIT" : "EXEC");
			printf("\"comm\":\"%s\",", e->comm);
			printf("\"pid\":%d,", e->pid);
			printf("\"ppid\":%d", e->ppid);
			
			if (e->exit_event) {
				printf(",\"exit_code\":%u", e->exit_code);
				if (e->duration_ns)
					printf(",\"duration_ms\":%llu", e->duration_ns / 1000000);
				
				// Flush all pending FILE_OPEN aggregations for this PID
				flush_pid_file_opens(e->pid, timestamp_ns);
			} else {
				printf(",\"filename\":\"%s\"", e->filename);
			}
			printf("}\n");
			break;
			
		case EVENT_TYPE_BASH_READLINE:
			// For bash readline events, always report immediately
			printf("{");
			printf("\"timestamp\":%llu,", timestamp_ns);
			printf("\"event\":\"BASH_READLINE\",");
			printf("\"comm\":\"%s\",", e->comm);
			printf("\"pid\":%d,", e->pid);
			printf("\"command\":\"%s\"", e->command);
			printf("}\n");
			break;
			
		case EVENT_TYPE_FILE_OPERATION:
			// Only handle FILE_OPEN events, skip FILE_CLOSE
			if (!e->file_op.is_open) {
				break;
			}
			
			// Get count for this FILE_OPEN operation
			uint32_t count = get_file_open_count(e, timestamp_ns);
			
			// Skip if this is a duplicate (count == 0)
			if (count == 0) {
				break;
			}
			
			// Report the FILE_OPEN event with count
			print_file_open_event(e, timestamp_ns, count, NULL);
			break;
			
		default:
			// For unknown events, always report immediately
			printf("{");
			printf("\"timestamp\":%llu,", timestamp_ns);
			printf("\"event\":\"UNKNOWN\",");
			printf("\"event_type\":%d", e->type);
			printf("}\n");
			break;
	}

	return 0;
}

int main(int argc, char **argv)
{
	struct ring_buffer *rb = NULL;
	struct process_bpf *skel;
	int err;

	/* Parse command line arguments */
	err = argp_parse(&argp, argc, argv, 0, NULL, NULL);
	if (err)
		return err;

	/* filter_mode is set via -m flag or -a flag, defaults to FILTER_MODE_FILTER */

	/* Set up libbpf errors and debug info callback */
	libbpf_set_print(libbpf_print_fn);

	/* Cleaner handling of Ctrl-C */
	signal(SIGINT, sig_handler);
	signal(SIGTERM, sig_handler);

	/* Load and verify BPF application */
	skel = process_bpf__open();
	if (!skel) {
		fprintf(stderr, "Failed to open and load BPF skeleton\n");
		return 1;
	}

	/* Parameterize BPF code with minimum duration and filter mode */
	skel->rodata->min_duration_ns = env.min_duration_ms * 1000000ULL;
	skel->rodata->filter_mode = env.filter_mode;

	/* Setup command filters if using filter mode */
	if (env.filter_mode == FILTER_MODE_FILTER) {
		err = setup_command_filters(skel, env.command_list, env.command_count);
		if (err) {
			fprintf(stderr, "Failed to setup command filters\n");
			goto cleanup;
		}
	}
	/* Load & verify BPF programs */
	err = process_bpf__load(skel);
	if (err) {
		fprintf(stderr, "Failed to load and verify BPF skeleton\n");
		goto cleanup;
	}

	/* Populate initial PIDs from existing processes */
	pid_t *tracked_pids_array;
	int tracked_count = populate_initial_pids(skel, env.command_list, env.command_count, env.filter_mode, &tracked_pids_array);
	if (tracked_count < 0) {
		fprintf(stderr, "Failed to populate initial PIDs\n");
		goto cleanup;
	}
	
	/* Output configuration as JSON */
	printf("Config: filter_mode=%d, min_duration_ms=%ld, commands=%d, pid=%d, initial_tracked_pids=%d\n", 
	       env.filter_mode, env.min_duration_ms, env.command_count, env.pid, tracked_count);

	/* Attach tracepoints */
	err = process_bpf__attach(skel);
	if (err) {
		fprintf(stderr, "Failed to attach BPF skeleton\n");
		goto cleanup;
	}

	/* Set up ring buffer polling */
	rb = ring_buffer__new(bpf_map__fd(skel->maps.rb), handle_event, NULL, NULL);
	if (!rb) {
		err = -1;
		fprintf(stderr, "Failed to create ring buffer\n");
		goto cleanup;
	}



	/* Process events */
	while (!exiting) {
		err = ring_buffer__poll(rb, 100 /* timeout, ms */);
		/* Ctrl-C will cause -EINTR */
		if (err == -EINTR) {
			err = 0;
			break;
		}
		if (err < 0) {
			printf("Error polling perf buffer: %d\n", err);
			break;
		}
	}

cleanup:
	/* Clean up */
	ring_buffer__free(rb);
	process_bpf__destroy(skel);
	
	/* Free allocated command strings */
	for (int i = 0; i < env.command_count; i++) {
		free(env.command_list[i]);
	}
	
	/* Clean up FILE_OPEN deduplication tracking */
	hash_count = 0;

	return err < 0 ? -err : 0;
}
