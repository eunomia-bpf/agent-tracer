/* SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause) */
/* Copyright (c) 2020 Facebook */
#ifndef __PROCESS_H
#define __PROCESS_H

#define TASK_COMM_LEN 16
#define MAX_FILENAME_LEN 127
#define MAX_COMMAND_FILTERS 10
#define MAX_TRACKED_PIDS 1024
#define MAX_COMMAND_LEN 256

enum event_type {
	EVENT_TYPE_PROCESS = 0,
	EVENT_TYPE_BASH_READLINE = 1,
};

struct event {
	enum event_type type;
	int pid;
	int ppid;
	unsigned exit_code;
	unsigned long long duration_ns;
	char comm[TASK_COMM_LEN];
	union {
		char filename[MAX_FILENAME_LEN];     /* for process events */
		char command[MAX_COMMAND_LEN];       /* for bash readline events */
	};
	bool exit_event;
};

struct command_filter {
	char comm[TASK_COMM_LEN];
};

struct pid_info {
	pid_t pid;
	pid_t ppid;
	bool is_tracked;
};

#endif /* __PROCESS_H */
