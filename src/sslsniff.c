// SPDX-License-Identifier: (LGPL-2.1 OR BSD-2-Clause)
// Copyright (c) 2023 Yusheng Zheng
//
// Based on sslsniff from BCC by Adrian Lopez & Mark Drayton.
// 15-Aug-2023   Yusheng Zheng   Created this.
#include <argp.h>
#include <bpf/bpf.h>
#include <bpf/libbpf.h>
#include <ctype.h>
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>

#include "sslsniff.skel.h"
#include "sslsniff.h"

#define INVALID_UID -1
#define INVALID_PID -1
#define DEFAULT_BUFFER_SIZE 8192

#define __ATTACH_UPROBE(skel, binary_path, sym_name, prog_name, is_retprobe)   \
	do {                                                                       \
	  LIBBPF_OPTS(bpf_uprobe_opts, uprobe_opts, .func_name = #sym_name,        \
				  .retprobe = is_retprobe);                                    \
	  skel->links.prog_name = bpf_program__attach_uprobe_opts(                 \
		  skel->progs.prog_name, env.pid, binary_path, 0, &uprobe_opts);       \
	} while (false)

#define __CHECK_PROGRAM(skel, prog_name)               \
	do {                                               \
	  if (!skel->links.prog_name) {                    \
		perror("no program attached for " #prog_name); \
		return -errno;                                 \
	  }                                                \
	} while (false)

#define __ATTACH_UPROBE_CHECKED(skel, binary_path, sym_name, prog_name,     \
								is_retprobe)                                \
	do {                                                                    \
	  __ATTACH_UPROBE(skel, binary_path, sym_name, prog_name, is_retprobe); \
	  __CHECK_PROGRAM(skel, prog_name);                                     \
	} while (false)

#define ATTACH_UPROBE_CHECKED(skel, binary_path, sym_name, prog_name)     \
	__ATTACH_UPROBE_CHECKED(skel, binary_path, sym_name, prog_name, false)
#define ATTACH_URETPROBE_CHECKED(skel, binary_path, sym_name, prog_name)  \
	__ATTACH_UPROBE_CHECKED(skel, binary_path, sym_name, prog_name, true)

volatile sig_atomic_t exiting = 0;

const char *argp_program_version = "sslsniff 0.1";
const char *argp_program_bug_address = "https://github.com/iovisor/bcc/tree/master/libbpf-tools";
const char argp_program_doc[] =
	"Sniff SSL data and output in JSON format.\n"
	"\n"
	"USAGE: sslsniff [OPTIONS]\n"
	"\n"
	"OUTPUT: Each SSL event is output as a JSON object on a separate line.\n"
	"eBPF capture is limited to 32KB per event due to kernel constraints.\n"
	"\n"
	"EXAMPLES:\n"
	"    ./sslsniff              # sniff OpenSSL and GnuTLS functions\n"
	"    ./sslsniff -p 181       # sniff PID 181 only\n"
	"    ./sslsniff -u 1000      # sniff only UID 1000\n"
	"    ./sslsniff -c curl      # sniff curl command only\n"
	"    ./sslsniff --no-openssl # don't show OpenSSL calls\n"
	"    ./sslsniff --no-gnutls  # don't show GnuTLS calls\n"
	"    ./sslsniff --no-nss     # don't show NSS calls\n"
	"    ./sslsniff --hexdump    # include data_hex field with hex data\n"
	"    ./sslsniff -x           # include uid and tid fields\n"
	"    ./sslsniff -l           # include latency_ms field\n"
	"    ./sslsniff -l --handshake  # include handshake latency\n"
	"    ./sslsniff --extra-lib openssl:/path/libssl.so.1.1 # sniff extra "
	"library\n";

struct env {
	pid_t pid;
	int uid;
	bool extra;
	char *comm;
	bool openssl;
	bool gnutls;
	bool nss;
	bool hexdump;
	bool latency;
	bool handshake;
	char *extra_lib;
} env = {
	.uid = INVALID_UID,
	.pid = INVALID_PID,
	.openssl = true,
	.gnutls = false,
	.nss = false,
	.comm = NULL,
};

#define HEXDUMP_KEY 1000
#define HANDSHAKE_KEY 1002
#define EXTRA_LIB_KEY 1003

static const struct argp_option opts[] = {
	{"pid", 'p', "PID", 0, "Sniff this PID only."},
	{"uid", 'u', "UID", 0, "Sniff this UID only."},
	{"extra", 'x', NULL, 0, "Show extra fields (UID, TID)"},
	{"comm", 'c', "COMMAND", 0, "Sniff only commands matching string."},
	{"no-openssl", 'o', NULL, 0, "Do not show OpenSSL calls."},
	{"no-gnutls", 'g', NULL, 0, "Do not show GnuTLS calls."},
	{"no-nss", 'n', NULL, 0, "Do not show NSS calls."},
	{"hexdump", HEXDUMP_KEY, NULL, 0,
	 "Show data as hexdump instead of trying to decode it as UTF-8"},
	{"latency", 'l', NULL, 0, "Show function latency"},
	{"handshake", HANDSHAKE_KEY, NULL, 0,
	 "Show SSL handshake latency, enabled only if latency option is on."},
	{"verbose", 'v', NULL, 0, "Verbose debug output"},
	{ NULL, 'h', NULL, OPTION_HIDDEN, "Show the full help" },
	{},
};

static bool verbose = false;

static error_t parse_arg(int key, char *arg, struct argp_state *state) {
	switch (key) {
	case 'p':
		env.pid = atoi(arg);
		break;
	case 'u':
		env.uid = atoi(arg);
		break;
	case 'x':
		env.extra = true;
		break;
	case 'c':
		env.comm = strdup(arg);
		break;
	case 'o':
		env.openssl = false;
		break;
	case 'g':
		env.gnutls = false;
		break;
	case 'n':
		env.nss = false;
		break;
	case 'l':
		env.latency = true;
		break;
	case 'v':
		verbose = true;
		break;
	case HEXDUMP_KEY:
		env.hexdump = true;
		break;
	case HANDSHAKE_KEY:
		env.handshake = true;
		break;
	case 'h':
		argp_state_help(state, stderr, ARGP_HELP_STD_HELP);
		break;
	default:
		return ARGP_ERR_UNKNOWN;
	}
	return 0;
}

#define PERF_POLL_TIMEOUT_MS 100
#define warn(...) fprintf(stderr, __VA_ARGS__)

static struct argp argp = {
	opts,
	parse_arg,
	NULL,
	argp_program_doc
};

static int libbpf_print_fn(enum libbpf_print_level level, const char *format,
						   va_list args) {
	if (level == LIBBPF_DEBUG && !verbose)
		return 0;
	return vfprintf(stderr, format, args);
}

/* handle_lost_events removed - ring buffer doesn't have lost events like perf buffer */

static void sig_int(int signo) { 
	exiting = 1;
}

int attach_openssl(struct sslsniff_bpf *skel, const char *lib) {
	ATTACH_UPROBE_CHECKED(skel, lib, SSL_write, probe_SSL_rw_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, SSL_write, probe_SSL_write_exit);
	ATTACH_UPROBE_CHECKED(skel, lib, SSL_read, probe_SSL_rw_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, SSL_read, probe_SSL_read_exit);

	ATTACH_UPROBE_CHECKED(skel, lib, SSL_write_ex, probe_SSL_write_ex_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, SSL_write_ex, probe_SSL_write_ex_exit);
	ATTACH_UPROBE_CHECKED(skel, lib, SSL_read_ex, probe_SSL_read_ex_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, SSL_read_ex, probe_SSL_read_ex_exit);

	if (env.latency && env.handshake) {
		ATTACH_UPROBE_CHECKED(skel, lib, SSL_do_handshake,
							probe_SSL_do_handshake_enter);
		ATTACH_URETPROBE_CHECKED(skel, lib, SSL_do_handshake,
								probe_SSL_do_handshake_exit);
	}

	return 0;
}

int attach_gnutls(struct sslsniff_bpf *skel, const char *lib) {
	ATTACH_UPROBE_CHECKED(skel, lib, gnutls_record_send, probe_SSL_rw_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, gnutls_record_send, probe_SSL_write_exit);
	ATTACH_UPROBE_CHECKED(skel, lib, gnutls_record_recv, probe_SSL_rw_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, gnutls_record_recv, probe_SSL_read_exit);

	return 0;
}

int attach_nss(struct sslsniff_bpf *skel, const char *lib) {
	ATTACH_UPROBE_CHECKED(skel, lib, PR_Write, probe_SSL_rw_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, PR_Write, probe_SSL_write_exit);
	ATTACH_UPROBE_CHECKED(skel, lib, PR_Send, probe_SSL_rw_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, PR_Send, probe_SSL_write_exit);
	ATTACH_UPROBE_CHECKED(skel, lib, PR_Read, probe_SSL_rw_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, PR_Read, probe_SSL_read_exit);
	ATTACH_UPROBE_CHECKED(skel, lib, PR_Recv, probe_SSL_rw_enter);
	ATTACH_URETPROBE_CHECKED(skel, lib, PR_Recv, probe_SSL_read_exit);

	return 0;
}

/*
 * Find the path of a library using ldconfig.
 */
char *find_library_path(const char *libname) {
	char cmd[128];
	static char path[512];
	FILE *fp;

	// Construct the ldconfig command with grep
	snprintf(cmd, sizeof(cmd), "ldconfig -p | grep %s", libname);

	// Execute the command and read the output
	fp = popen(cmd, "r");
	if (fp == NULL) {
		perror("Failed to run ldconfig");
		return NULL;
	}

	// Read the first line of output which should have the library path
	if (fgets(path, sizeof(path) - 1, fp) != NULL) {
		// Extract the path from the ldconfig output
		char *start = strrchr(path, '>');
		if (start && *(start + 1) == ' ') {
			memmove(path, start + 2, strlen(start + 2) + 1);
			char *end = strchr(path, '\n');
			if (end) {
				*end = '\0';  // Null-terminate the path
			}
			pclose(fp);
			return path;
		}
	}

	pclose(fp);
	return NULL;
}

// Global buffers allocated once and reused
static char *event_buf = NULL;
static char *hex_buf = NULL;

void buf_to_hex(const uint8_t *buf, size_t len, char *hex_str) {
	for (size_t i = 0; i < len; i++) {
		sprintf(hex_str + 2 * i, "%02x", buf[i]);
	}
}

// Function to print the event from the perf buffer in JSON format
void print_event(struct probe_SSL_data_t *event, const char *evt) {
	static unsigned long long start = 0;  // Use static to retain value across function calls
	unsigned int buf_size;

	// Safety check for global buffers
	if (!event_buf || !hex_buf) {
		fprintf(stderr, "Error: global buffers not allocated\n");
		return;
	}

	// eBPF captures up to MAX_BUF_SIZE bytes per event
	if (event->len <= MAX_BUF_SIZE) {
		buf_size = event->len;
	} else {
		buf_size = MAX_BUF_SIZE;
	}

	if (event->buf_filled == 1 && buf_size > 0) {
		// Additional safety check to prevent buffer overflow
		if (buf_size > MAX_BUF_SIZE) {
			buf_size = MAX_BUF_SIZE;
		}
		memcpy(event_buf, event->buf, buf_size);
		event_buf[buf_size] = '\0';  // Null terminate
	} else {
		buf_size = 0;
	}

	if (env.comm && strcmp(env.comm, event->comm) != 0) {
		return;
	}

	if (start == 0) {
		start = event->timestamp_ns;
	}
	double time_s = (double)(event->timestamp_ns - start) / 1000000000;

	char *rw_event[] = {
		"READ/RECV",
		"WRITE/SEND",
		"HANDSHAKE"
	};

	// Start JSON object
	printf("{");
	
	// Basic fields
	printf("\"function\":\"%s\",", rw_event[event->rw]);
	printf("\"time_s\":%.9f,", time_s);
	printf("\"timestamp_ns\":%llu,", event->timestamp_ns);
	printf("\"comm\":\"%s\",", event->comm);
	printf("\"pid\":%d,", event->pid);
	printf("\"len\":%d", event->len);

	// Extra fields if enabled
	if (env.extra) {
		printf(",\"uid\":%d,\"tid\":%d", event->uid, event->tid);
	}

	// Latency field if enabled
	if (env.latency && event->delta_ns) {
		printf(",\"latency_ms\":%.3f", (double)event->delta_ns / 1000000);
	}

	// Handshake field
	printf(",\"is_handshake\":%s", event->is_handshake ? "true" : "false");

	// Data field
	if (buf_size > 0) {
		if (env.hexdump) {
			// Output as hex string using pre-allocated buffer
			buf_to_hex((uint8_t *)event_buf, buf_size, hex_buf);
			printf(",\"data_hex\":\"%s\"", hex_buf);
		} else {
			// Escape the buffer data for JSON with UTF-8 support
			printf(",\"data\":\"");
			for (unsigned int i = 0; i < buf_size; i++) {
				unsigned char c = event_buf[i];
				if (c == '"' || c == '\\') {
					printf("\\%c", c);
				} else if (c == '\n') {
					printf("\\n");
				} else if (c == '\r') {
					printf("\\r");
				} else if (c == '\t') {
					printf("\\t");
				} else if (c == '\b') {
					printf("\\b");
				} else if (c == '\f') {
					printf("\\f");
				} else if (c >= 32 && c <= 126) {
					// ASCII printable characters
					printf("%c", c);
				} else if (c >= 128) {
					// UTF-8 multi-byte sequence - pass through as-is
					printf("%c", c);
				} else {
					// Control characters (0-31, 127)
					printf("\\u%04x", c);
				}
			}
			printf("\"");
		}
		
		// Add truncated info if data was truncated
		if (buf_size < event->len) {
			printf(",\"truncated\":true,\"bytes_lost\":%d", event->len - buf_size);
		} else {
			printf(",\"truncated\":false");
		}
	} else {
		printf(",\"data\":null,\"truncated\":false");
	}

	// Close JSON object
	printf("}\n");
}

static int handle_event(void *ctx, void *data, size_t data_sz) {
	struct probe_SSL_data_t *e = data;
	if (e->is_handshake) {
		print_event(e, "ringbuf_SSL_do_handshake");
	} else {
		print_event(e, "ringbuf_SSL_rw");
	}
	return 0;
}

int main(int argc, char **argv) {
	LIBBPF_OPTS(bpf_object_open_opts, open_opts);
	struct sslsniff_bpf *obj = NULL;
	struct ring_buffer *rb = NULL;
	int err;

	err = argp_parse(&argp, argc, argv, 0, NULL, NULL);
	if (err)
		return err;

	libbpf_set_print(libbpf_print_fn);

	obj = sslsniff_bpf__open_opts(&open_opts);
	if (!obj) {
		warn("failed to open BPF object\n");
		goto cleanup;
	}

	obj->rodata->targ_uid = env.uid;
	obj->rodata->targ_pid = env.pid == INVALID_PID ? 0 : env.pid;

	err = sslsniff_bpf__load(obj);
	if (err) {
		warn("failed to load BPF object: %d\n", err);
		goto cleanup;
	}

	// Allocate global buffers once
	event_buf = malloc(MAX_BUF_SIZE + 1);
	if (!event_buf) {
		warn("failed to allocate event buffer\n");
		err = -ENOMEM;
		goto cleanup;
	}

	hex_buf = malloc(MAX_BUF_SIZE * 2 + 1);
	if (!hex_buf) {
		warn("failed to allocate hex buffer\n");
		err = -ENOMEM;
		goto cleanup;
	}

	if (env.openssl) {
		char *openssl_path = find_library_path("libssl.so");
		if (verbose) {
			fprintf(stderr, "OpenSSL path: %s\n", openssl_path ? openssl_path : "not found");
		}
		if (openssl_path) {
			attach_openssl(obj, openssl_path);
		} else {
			warn("OpenSSL library not found\n");
		}
	}
	if (env.gnutls) {
		char *gnutls_path = find_library_path("libgnutls.so");
		if (verbose) {
			fprintf(stderr, "GnuTLS path: %s\n", gnutls_path ? gnutls_path : "not found");
		}
		if (gnutls_path) {
			attach_gnutls(obj, gnutls_path);
		} else {
			warn("GnuTLS library not found\n");
		}
	}
	if (env.nss) {
		char *nss_path = find_library_path("libnspr4.so");
		if (verbose) {
			fprintf(stderr, "NSS path: %s\n", nss_path ? nss_path : "not found");
		}
		if (nss_path) {
			attach_nss(obj, nss_path);
		} else {
			warn("NSS library not found\n");
		}
	}

	rb = ring_buffer__new(bpf_map__fd(obj->maps.rb), handle_event, NULL, NULL);
	if (!rb) {
		err = -errno;
		warn("failed to open ring buffer: %d\n", err);
		goto cleanup;
	}

	if (signal(SIGINT, sig_int) == SIG_ERR) {
		warn("can't set signal handler: %s\n", strerror(errno));
		err = 1;
		goto cleanup;
	}

	while (!exiting) {
		err = ring_buffer__poll(rb, PERF_POLL_TIMEOUT_MS);
		if (err < 0 && err != -EINTR) {
			warn("error polling ring buffer: %s\n", strerror(-err));
			goto cleanup;
		}
		err = 0;
	}

cleanup:
	if (event_buf) {
		free(event_buf);
		event_buf = NULL;
	}
	if (hex_buf) {
		free(hex_buf);
		hex_buf = NULL;
	}
	ring_buffer__free(rb);
	sslsniff_bpf__destroy(obj);
	return err != 0;
}
