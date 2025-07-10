// SPDX-License-Identifier: GPL-2.0 OR BSD-3-Clause
/* Copyright (c) 2020 Facebook */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>
#include "process.h"

char LICENSE[] SEC("license") = "Dual BSD/GPL";

struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 8192);
	__type(key, pid_t);
	__type(value, u64);
} exec_start SEC(".maps");

struct {
	__uint(type, BPF_MAP_TYPE_RINGBUF);
	__uint(max_entries, 256 * 1024);
} rb SEC(".maps");

const struct command_filter command_filters[MAX_COMMAND_FILTERS] = {0};

/* Map to store tracked PIDs */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, MAX_TRACKED_PIDS);
	__type(key, pid_t);
	__type(value, struct pid_info);
} tracked_pids SEC(".maps");

const volatile unsigned long long min_duration_ns = 0;
const volatile bool trace_all_processes = false;

/* Context structure for filter checking */
struct filter_check_ctx {
	const char *comm;
	pid_t pid;
	pid_t ppid;
	bool found_match;
};

/* Callback function for filter checking loop */
static int check_filter_callback(__u32 index, void *ctx)
{
	struct filter_check_ctx *filter_ctx = (struct filter_check_ctx *)ctx;
	const struct command_filter *filter;
	__u32 i = index;
	
	filter = &command_filters[i];
	if (filter->comm[0] == '\0')
		return 0; /* continue loop - empty filter */

	/* Check if command matches the filter string exactly */
	if (bpf_strncmp(filter_ctx->comm, TASK_COMM_LEN, filter->comm) == 0) {
		/* Add this PID to tracked list */
		struct pid_info new_pid_info = {
			.pid = filter_ctx->pid,
			.ppid = filter_ctx->ppid,
			.is_tracked = true
		};
		bpf_map_update_elem(&tracked_pids, &filter_ctx->pid, &new_pid_info, BPF_ANY);
		filter_ctx->found_match = true;
		return 1; /* stop loop */
	}
	
	return 0; /* continue loop */
}

static __always_inline bool should_trace_process(const char *comm, pid_t pid, pid_t ppid)
{
	struct pid_info *parent_info;

	/* If tracing all processes, always return true */
	if (trace_all_processes)
		return true;

	/* First check if this PID is already being tracked */
	struct pid_info *pid_info = bpf_map_lookup_elem(&tracked_pids, &pid);
	if (pid_info && pid_info->is_tracked) {
		return true;
	}

	/* Check if parent PID is being tracked */
	parent_info = bpf_map_lookup_elem(&tracked_pids, &ppid);
	if (parent_info && parent_info->is_tracked) {
		/* Add this PID to tracked list as child of tracked parent */
		struct pid_info new_pid_info = {
			.pid = pid,
			.ppid = ppid,
			.is_tracked = true
		};
		bpf_map_update_elem(&tracked_pids, &pid, &new_pid_info, BPF_ANY);
		return true;
	}

	/* Check if process command matches any configured filter */
	struct filter_check_ctx filter_ctx = {
		.comm = comm,
		.pid = pid,
		.ppid = ppid,
		.found_match = false
	};
	
	bpf_loop(MAX_COMMAND_FILTERS, check_filter_callback, &filter_ctx, 0);
	
	return filter_ctx.found_match;
}

/* Helper function to check if any PIDs are being tracked */
static __always_inline bool has_tracked_pids(void)
{
	struct pid_info *info;
	pid_t test_pid = 1; /* Use a dummy PID for iteration */
	
	/* If tracing all processes, always allow bash tracing */
	if (trace_all_processes)
		return true;
	
	/* Check if tracked_pids map has any entries */
	/* This is a simple heuristic - we could make it more sophisticated */
	return bpf_map_lookup_elem(&tracked_pids, &test_pid) != NULL;
}

/* Bash readline uretprobe handler */
SEC("uretprobe//usr/bin/bash:readline")
int BPF_URETPROBE(bash_readline, const void *ret)
{
	struct event *e;
	char comm[TASK_COMM_LEN];
	u32 pid;

	if (!ret)
		return 0;

	/* Check if this is actually bash */
	bpf_get_current_comm(&comm, sizeof(comm));
	if (comm[0] != 'b' || comm[1] != 'a' || comm[2] != 's' || comm[3] != 'h' || comm[4] != 0)
		return 0;

	/* Only trace bash commands if we have tracked PIDs or tracing all */
	if (!has_tracked_pids())
		return 0;

	pid = bpf_get_current_pid_tgid() >> 32;

	/* If not tracing all processes, check if this specific PID should be traced */
	if (!trace_all_processes) {
		struct pid_info *pid_info = bpf_map_lookup_elem(&tracked_pids, &pid);
		if (!pid_info || !pid_info->is_tracked)
			return 0;
	}

	/* Reserve sample from BPF ringbuf */
	e = bpf_ringbuf_reserve(&rb, sizeof(*e), 0);
	if (!e)
		return 0;

	/* Fill out the sample with bash readline data */
	e->type = EVENT_TYPE_BASH_READLINE;
	e->pid = pid;
	e->ppid = 0; /* Not relevant for bash commands */
	e->exit_code = 0;
	e->duration_ns = 0;
	e->exit_event = false;
	bpf_get_current_comm(&e->comm, sizeof(e->comm));
	bpf_probe_read_user_str(&e->command, sizeof(e->command), ret);

	/* Submit to user-space for post-processing */
	bpf_ringbuf_submit(e, 0);
	return 0;
}

SEC("tp/sched/sched_process_exec")
int handle_exec(struct trace_event_raw_sched_process_exec *ctx)
{
	struct task_struct *task;
	unsigned fname_off;
	struct event *e;
	pid_t pid;
	u64 ts;
	char comm[TASK_COMM_LEN];

	/* Get process info */
	pid = bpf_get_current_pid_tgid() >> 32;
	task = (struct task_struct *)bpf_get_current_task();
	bpf_get_current_comm(&comm, sizeof(comm));
	pid_t ppid = BPF_CORE_READ(task, real_parent, tgid);

	/* Check if we should trace this process */
	if (!should_trace_process(comm, pid, ppid))
		return 0;

	/* remember time exec() was executed for this PID */
	pid = bpf_get_current_pid_tgid() >> 32;
	ts = bpf_ktime_get_ns();
	bpf_map_update_elem(&exec_start, &pid, &ts, BPF_ANY);

	/* don't emit exec events when minimum duration is specified */
	if (min_duration_ns)
		return 0;

	/* reserve sample from BPF ringbuf */
	e = bpf_ringbuf_reserve(&rb, sizeof(*e), 0);
	if (!e)
		return 0;

	/* fill out the sample with data */
	task = (struct task_struct *)bpf_get_current_task();

	e->type = EVENT_TYPE_PROCESS;
	e->exit_event = false;
	e->pid = pid;
	e->ppid = BPF_CORE_READ(task, real_parent, tgid);
	bpf_get_current_comm(&e->comm, sizeof(e->comm));

	fname_off = ctx->__data_loc_filename & 0xFFFF;
	bpf_probe_read_str(&e->filename, sizeof(e->filename), (void *)ctx + fname_off);

	/* successfully submit it to user-space for post-processing */
	bpf_ringbuf_submit(e, 0);
	return 0;
}

SEC("tp/sched/sched_process_exit")
int handle_exit(struct trace_event_raw_sched_process_template* ctx)
{
	struct task_struct *task;
	struct event *e;
	pid_t pid, tid;
	u64 id, ts, *start_ts, duration_ns = 0;
	
	/* get PID and TID of exiting thread/process */
	id = bpf_get_current_pid_tgid();
	pid = id >> 32;
	tid = (u32)id;

	/* ignore thread exits */
	if (pid != tid)
		return 0;

	/* Check if this PID is being tracked (or trace all processes) */
	if (!trace_all_processes) {
		struct pid_info *pid_info = bpf_map_lookup_elem(&tracked_pids, &pid);
		if (!pid_info || !pid_info->is_tracked)
			return 0;
	}

	/* if we recorded start of the process, calculate lifetime duration */
	start_ts = bpf_map_lookup_elem(&exec_start, &pid);
	if (start_ts)
		duration_ns = bpf_ktime_get_ns() - *start_ts;
	else if (min_duration_ns)
		return 0;
	bpf_map_delete_elem(&exec_start, &pid);

	/* if process didn't live long enough, return early */
	if (min_duration_ns && duration_ns < min_duration_ns)
		return 0;

	/* reserve sample from BPF ringbuf */
	e = bpf_ringbuf_reserve(&rb, sizeof(*e), 0);
	if (!e)
		return 0;

	/* fill out the sample with data */
	task = (struct task_struct *)bpf_get_current_task();

	e->type = EVENT_TYPE_PROCESS;
	e->exit_event = true;
	e->duration_ns = duration_ns;
	e->pid = pid;
	e->ppid = BPF_CORE_READ(task, real_parent, tgid);
	e->exit_code = (BPF_CORE_READ(task, exit_code) >> 8) & 0xff;
	bpf_get_current_comm(&e->comm, sizeof(e->comm));

	/* Remove from tracked PIDs on exit if not tracing all */
	if (!trace_all_processes)
		bpf_map_delete_elem(&tracked_pids, &pid);

	/* send data to user-space for post-processing */
	bpf_ringbuf_submit(e, 0);
	return 0;
}

