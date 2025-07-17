/* SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause) */
#ifndef __PROCESS_UTILS_H
#define __PROCESS_UTILS_H

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <dirent.h>
#include <unistd.h>
#include <stdbool.h>
#include <stdint.h>

// Forward declarations for BPF types when not in test mode
#ifndef BPF_ANY
#include <bpf/libbpf.h>
typedef uint32_t __u32;
#endif

#include "process.h"

static int read_proc_comm(pid_t pid, char *comm, size_t size)
{
	char path[256];
	FILE *f;
	
	snprintf(path, sizeof(path), "/proc/%d/comm", pid);
	f = fopen(path, "r");
	if (!f)
		return -1;
	
	if (fgets(comm, size, f)) {
		/* Remove trailing newline */
		char *newline = strchr(comm, '\n');
		if (newline)
			*newline = '\0';
	} else {
		fclose(f);
		return -1;
	}
	
	fclose(f);
	return 0;
}

static int read_proc_ppid(pid_t pid, pid_t *ppid)
{
	char path[256];
	FILE *f;
	char line[256];
	
	snprintf(path, sizeof(path), "/proc/%d/stat", pid);
	f = fopen(path, "r");
	if (!f)
		return -1;
	
	if (fgets(line, sizeof(line), f)) {
		/* Parse the stat line to get ppid (4th field) */
		char *token = strtok(line, " ");
		for (int i = 0; i < 3 && token; i++) {
			token = strtok(NULL, " ");
		}
		if (token) {
			*ppid = (pid_t)strtol(token, NULL, 10);
		} else {
			fclose(f);
			return -1;
		}
	} else {
		fclose(f);
		return -1;
	}
	
	fclose(f);
	return 0;
}

static bool command_matches_filter(const char *comm, const char *filter)
{
	return strstr(comm, filter) != NULL;
}

/* Count and print processes that match the given command filters */
static int count_matching_processes(char **command_list, int command_count, bool trace_all)
{
	DIR *proc_dir;
	struct dirent *entry;
	pid_t pid, ppid;
	char comm[TASK_COMM_LEN];
	int matching_count = 0;
	
	proc_dir = opendir("/proc");
	if (!proc_dir) {
		fprintf(stderr, "Failed to open /proc directory\n");
		return -1;
	}
	
	if (trace_all) {
		printf("Tracing all processes (no filter specified)\n");
	} else {
		printf("Scanning existing processes for matching commands...\n");
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
			if (!trace_all) {
				printf("  Found matching process: PID=%d, PPID=%d, COMM=%s\n", 
					pid, ppid, comm);
			}
			matching_count++;
		}
	}
	
	closedir(proc_dir);
	printf("Initially tracking %d processes\n", matching_count);
	return matching_count;
}



#endif /* __PROCESS_UTILS_H */ 