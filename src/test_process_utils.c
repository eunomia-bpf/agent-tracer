#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <assert.h>
#include <sys/wait.h>
#include <stdbool.h>
#include <stdint.h>

// BPF type definitions for testing
typedef uint32_t __u32;
typedef uint64_t __u64;
#define BPF_ANY 0

#include "process.h"



#include "process_utils.h"

// Test colors for output
#define RESET   "\033[0m"
#define RED     "\033[31m"
#define GREEN   "\033[32m"
#define YELLOW  "\033[33m"
#define BLUE    "\033[34m"

static int tests_passed = 0;
static int tests_failed = 0;

void test_assert(bool condition, const char *test_name) {
    if (condition) {
        printf("[" GREEN "PASS" RESET "] %s\n", test_name);
        tests_passed++;
    } else {
        printf("[" RED "FAIL" RESET "] %s\n", test_name);
        tests_failed++;
    }
}

void test_read_proc_comm() {
    printf("\n" BLUE "Testing read_proc_comm function:" RESET "\n");
    
    char comm[TASK_COMM_LEN];
    pid_t current_pid = getpid();
    
    // Test reading current process command
    int result = read_proc_comm(current_pid, comm, sizeof(comm));
    test_assert(result == 0, "read_proc_comm should succeed for current process");
    test_assert(strlen(comm) > 0, "comm should not be empty");
    printf("  Current process comm: '%s'\n", comm);
    
    // Test reading init process (PID 1)
    result = read_proc_comm(1, comm, sizeof(comm));
    test_assert(result == 0, "read_proc_comm should succeed for init process");
    test_assert(strlen(comm) > 0, "init comm should not be empty");
    printf("  Init process comm: '%s'\n", comm);
    
    // Test with invalid PID
    result = read_proc_comm(999999, comm, sizeof(comm));
    test_assert(result == -1, "read_proc_comm should fail for invalid PID");
}

void test_read_proc_ppid() {
    printf("\n" BLUE "Testing read_proc_ppid function:" RESET "\n");
    
    pid_t ppid;
    pid_t current_pid = getpid();
    
    // Test reading current process parent PID
    int result = read_proc_ppid(current_pid, &ppid);
    test_assert(result == 0, "read_proc_ppid should succeed for current process");
    test_assert(ppid > 0, "ppid should be positive");
    printf("  Current process PPID: %d\n", ppid);
    
    // Test reading init process parent (should be 0)
    result = read_proc_ppid(1, &ppid);
    test_assert(result == 0, "read_proc_ppid should succeed for init process");
    printf("  Init process PPID: %d\n", ppid);
    
    // Test with invalid PID
    result = read_proc_ppid(999999, &ppid);
    test_assert(result == -1, "read_proc_ppid should fail for invalid PID");
}

void test_command_matches_filter() {
    printf("\n" BLUE "Testing command_matches_filter function:" RESET "\n");
    
    // Test exact matches
    test_assert(command_matches_filter("bash", "bash"), "exact match should work");
    test_assert(command_matches_filter("python3", "python"), "substring match should work");
    test_assert(command_matches_filter("node", "node"), "exact match with node should work");
    
    // Test non-matches
    test_assert(!command_matches_filter("bash", "python"), "non-match should return false");
    test_assert(!command_matches_filter("vim", "emacs"), "different commands should not match");
    
    // Test empty cases
    test_assert(!command_matches_filter("", "bash"), "empty comm should not match");
    test_assert(command_matches_filter("bash", ""), "empty filter should match (strstr behavior)");
    
    // Test case sensitivity
    test_assert(!command_matches_filter("BASH", "bash"), "case sensitivity should work");
    test_assert(command_matches_filter("bash", "bas"), "partial match should work");
}

void test_count_matching_processes() {
    printf("\n" BLUE "Testing count_matching_processes function:" RESET "\n");
    
    char *command_list[] = {"bash"};
    int command_count = 1;
    
    // Test with trace_all = true
    int result = count_matching_processes(command_list, command_count, true);
    test_assert(result >= 0, "count_matching_processes should succeed with trace_all=true");
    
    // Test with trace_all = false
    result = count_matching_processes(command_list, command_count, false);
    test_assert(result >= 0, "count_matching_processes should succeed with trace_all=false");
    
    // Test with empty command list
    result = count_matching_processes(NULL, 0, false);
    test_assert(result >= 0, "count_matching_processes should succeed with empty command list");
}



void test_integration() {
    printf("\n" BLUE "Testing integration scenario:" RESET "\n");
    
    // Fork a child process to test process tracking
    pid_t child_pid = fork();
    
    if (child_pid == 0) {
        // Child process - sleep briefly then exit
        usleep(100000); // 100ms
        exit(0);
    } else if (child_pid > 0) {
        // Parent process
        char comm[TASK_COMM_LEN];
        pid_t ppid;
        
        // Test reading child process info
        usleep(50000); // 50ms - let child start
        
        int result1 = read_proc_comm(child_pid, comm, sizeof(comm));
        int result2 = read_proc_ppid(child_pid, &ppid);
        
        // Wait for child to complete
        int status;
        waitpid(child_pid, &status, 0);
        
        test_assert(result1 == 0, "should read child process comm");
        test_assert(result2 == 0, "should read child process ppid");
        test_assert(ppid == getpid(), "child ppid should match parent pid");
        
        printf("  Child process: PID=%d, PPID=%d, COMM='%s'\n", child_pid, ppid, comm);
    } else {
        printf("  Fork failed, skipping integration test\n");
    }
}

void print_test_summary() {
    printf("\n" YELLOW "===== Test Summary =====" RESET "\n");
    printf("Tests passed: " GREEN "%d" RESET "\n", tests_passed);
    printf("Tests failed: " RED "%d" RESET "\n", tests_failed);
    printf("Total tests:  %d\n", tests_passed + tests_failed);
    
    if (tests_failed == 0) {
        printf(GREEN "All tests passed!" RESET "\n");
    } else {
        printf(RED "Some tests failed!" RESET "\n");
    }
}

int main() {
    printf(BLUE "===== Process Utils Test Suite =====" RESET "\n");
    printf("Testing functions from process_utils.h\n");
    
    test_read_proc_comm();
    test_read_proc_ppid();
    test_command_matches_filter();
    test_count_matching_processes();
    test_integration();
    
    print_test_summary();
    
    return (tests_failed > 0) ? 1 : 0;
} 