// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
/* Copyright (c) 2020 Facebook */
#include <argp.h>
#include <signal.h>
#include <stdio.h>
#include <time.h>
#include <sys/resource.h>
#include <bpf/libbpf.h>
#include <dirent.h>
#include "process.h"
#include "process.skel.h"
#include "process_utils.h"

#define MAX_COMMAND_LIST 256

static struct env {
	bool verbose;
	long min_duration_ms;
	char *command_list[MAX_COMMAND_LIST];
	int command_count;
	bool trace_all;
} env;

const char *argp_program_version = "process-tracer 1.0";
const char *argp_program_bug_address = "<bpf@vger.kernel.org>";
const char argp_program_doc[] =
"BPF process tracer with command filtering.\n"
"\n"
"It traces process start and exits for specified commands and their subprocesses.\n"
"Shows associated information (filename, process duration, PID and PPID, etc).\n"
"By default, traces ALL processes if no commands are specified.\n"
"\n"
"USAGE: ./process [-d <min-duration-ms>] [-c <command1,command2,...>] [-v]\n"
"\n"
"EXAMPLES:\n"
"  ./process                        # Trace all processes\n"
"  ./process -c \"claude,python\"  # Trace processes containing 'claude' or 'python'\n"
"  ./process -c \"ssh\" -d 1000   # Trace ssh processes lasting > 1 second\n";

static const struct argp_option opts[] = {
	{ "verbose", 'v', NULL, 0, "Verbose debug output" },
	{ "duration", 'd', "DURATION-MS", 0, "Minimum process duration (ms) to report" },
	{ "commands", 'c', "COMMAND-LIST", 0, "Comma-separated list of commands to trace (e.g., \"claude,python\"). If not specified, traces all processes." },
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
	case 'c':
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
static int populate_initial_pids(struct process_bpf *skel, char **command_list, int command_count, bool trace_all, pid_t **tracked_pids_out)
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
		
		bool should_track = trace_all;
		
		/* If not tracing all, check if this process matches any configured filter */
		if (!trace_all && command_list && command_count > 0) {
			should_track = false;
			for (int i = 0; i < command_count; i++) {
				if (command_matches_filter(comm, command_list[i])) {
					should_track = true;
					break;
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
			if (err && !trace_all) {  /* Don't spam errors when tracing all processes */
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
	struct tm *tm;
	char ts[32];
	time_t t;

	time(&t);
	tm = localtime(&t);
	strftime(ts, sizeof(ts), "%H:%M:%S", tm);

	printf("{");
	printf("\"type\":\"event\",");
	printf("\"timestamp\":\"%s\",", ts);
	
	switch (e->type) {
		case EVENT_TYPE_PROCESS:
			printf("\"event\":\"%s\",", e->exit_event ? "EXIT" : "EXEC");
			printf("\"comm\":\"%s\",", e->comm);
			printf("\"pid\":%d,", e->pid);
			printf("\"ppid\":%d", e->ppid);
			
			if (e->exit_event) {
				printf(",\"exit_code\":%u", e->exit_code);
				if (e->duration_ns)
					printf(",\"duration_ms\":%llu", e->duration_ns / 1000000);
			} else {
				printf(",\"filename\":\"%s\"", e->filename);
			}
			break;
			
		case EVENT_TYPE_BASH_READLINE:
			printf("\"event\":\"BASH_READLINE\",");
			printf("\"comm\":\"%s\",", e->comm);
			printf("\"pid\":%d,", e->pid);
			printf("\"command\":\"%s\"", e->command);
			break;
			
		case EVENT_TYPE_FILE_OPERATION:
			printf("\"event\":\"%s\",", e->file_op.is_open ? "FILE_OPEN" : "FILE_CLOSE");
			printf("\"comm\":\"%s\",", e->comm);
			printf("\"pid\":%d,", e->pid);
			if (e->file_op.is_open) {
				printf("\"filepath\":\"%s\",", e->file_op.filepath);
				printf("\"flags\":%d", e->file_op.flags);
			} else {
				printf("\"fd\":%d", e->file_op.fd);
			}
			break;
			
		default:
			printf("\"event\":\"UNKNOWN\",");
			printf("\"event_type\":%d", e->type);
			break;
	}
	
	printf("}\n");

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

	/* Determine if we should trace all processes (default if no commands specified) */
	env.trace_all = (env.command_count == 0);

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

	/* Parameterize BPF code with minimum duration parameter */
	skel->rodata->min_duration_ns = env.min_duration_ms * 1000000ULL;
	skel->rodata->trace_all_processes = env.trace_all;

	/* Setup command filters if not tracing all */
	if (!env.trace_all) {
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
	int tracked_count = populate_initial_pids(skel, env.command_list, env.command_count, env.trace_all, &tracked_pids_array);
	if (tracked_count < 0) {
		fprintf(stderr, "Failed to populate initial PIDs\n");
		goto cleanup;
	}
	
	/* Output configuration as JSON */
	printf("{\"type\":\"config\",\"trace_all\":%s,\"min_duration_ms\":%ld,\"commands\":[", 
	       env.trace_all ? "true" : "false", env.min_duration_ms);
	for (int i = 0; i < env.command_count; i++) {
		printf("\"%s\"%s", env.command_list[i], 
		       (i < env.command_count - 1) ? "," : "");
	}
	printf("],\"initial_tracked_pids\":[");
	for (int i = 0; i < tracked_count && i < MAX_TRACKED_PIDS; i++) {
		printf("%d%s", tracked_pids_array[i], (i < tracked_count - 1) ? "," : "");
	}
	printf("]}\n");

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

	return err < 0 ? -err : 0;
}
