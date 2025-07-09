// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
// Copyright (c) 2023 Yusheng Zheng
//
// Based on sslsniff from BCC by Adrian Lopez & Mark Drayton.
// 15-Aug-2023   Yusheng Zheng   Created this.
#ifndef __SSLSNIFF_H
#define __SSLSNIFF_H

#define MAX_BUF_SIZE (32 * 1024)  // 32KB eBPF buffer size (eBPF限制)
#define DEFAULT_USER_BUF_SIZE (512 * 1024)  // 512KB默认用户空间buffer size
#define TASK_COMM_LEN 16

struct probe_SSL_data_t {
    __u64 timestamp_ns;
    __u64 delta_ns;
    __u32 pid;
    __u32 tid;
    __u32 uid;
    __u32 len;
    int buf_filled;
    int rw;
    char comm[TASK_COMM_LEN];
    __u8 buf[MAX_BUF_SIZE];
    int is_handshake;
};

#endif /* __SSLSNIFF_H */
