#!/usr/bin/env python3
import subprocess
import time
import os
import shutil
import statistics

def run_command(cmd, cwd=None):
    """Run a command and return the result"""
    result = subprocess.run(cmd, shell=True, cwd=cwd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"Error running command: {cmd}")
        print(f"Error output: {result.stderr}")
        raise Exception(f"Command failed: {cmd}")
    return result

def clone_repo(target_dir):
    """Clone the repository to a target directory"""
    print(f"Cloning repository to {target_dir}...")
    cmd = f"git clone https://github.com/eunomia-bpf/bpf-developer-tutorial {target_dir}"
    run_command(cmd)
    print(f"Successfully cloned to {target_dir}")

def build_repo(repo_dir):
    """Build the repository and return the time taken"""
    print(f"Building in {repo_dir}...")
    
    # Change to repo directory and run the build command
    build_cmd = "claude --permission-mode acceptEdits -p \'write a script for cpufreq in bpftrace\'"
    
    start_time = time.time()
    run_command(build_cmd, cwd=repo_dir)
    end_time = time.time()
    
    elapsed_time = end_time - start_time
    print(f"Build completed in {elapsed_time:.2f} seconds")
    
    return elapsed_time

def cleanup_repo(repo_dir):
    """Remove the cloned repository directory"""
    print(f"Cleaning up {repo_dir}...")
    if os.path.exists(repo_dir):
        shutil.rmtree(repo_dir)
        print(f"Removed {repo_dir}")

def main():
    # Configuration
    num_builds = 3
    base_dir = "/root/yunwei37"
    repo_dirs = [f"{base_dir}/bpf-tutorial-{i+1}" for i in range(num_builds)]
    
    print("=== BPF Developer Tutorial Build Benchmark ===\n")
    
    # Phase 1: Prepare - Clone repositories
    print("Phase 1: Cloning repositories...")
    for repo_dir in repo_dirs:
        try:
            clone_repo(repo_dir)
        except Exception as e:
            print(f"Failed to clone to {repo_dir}: {e}")
            # Cleanup any partial clones
            for dir in repo_dirs:
                cleanup_repo(dir)
            return
    
    print("\nPhase 1 completed: All repositories cloned.\n")
    
    # Phase 2: Test - Build and measure time
    print("Phase 2: Building repositories and measuring time...")
    build_times = []
    
    for i, repo_dir in enumerate(repo_dirs):
        print(f"\nBuild {i+1}/{num_builds}:")
        try:
            build_time = build_repo(repo_dir)
            build_times.append(build_time)
        except Exception as e:
            print(f"Failed to build {repo_dir}: {e}")
            # Continue with other builds
    
    print("\nPhase 2 completed: All builds finished.\n")
    
    # Calculate and display results
    if build_times:
        print("=== Build Time Results ===")
        for i, time_taken in enumerate(build_times):
            print(f"Build {i+1}: {time_taken:.2f} seconds")
        
        avg_time = statistics.mean(build_times)
        print(f"\nAverage build time: {avg_time:.2f} seconds")
        
        if len(build_times) > 1:
            std_dev = statistics.stdev(build_times)
            print(f"Standard deviation: {std_dev:.2f} seconds")
            print(f"Min time: {min(build_times):.2f} seconds")
            print(f"Max time: {max(build_times):.2f} seconds")
    else:
        print("No successful builds to report.")
    
    # Phase 3: Cleanup - Remove cloned repositories
    print("\nPhase 3: Cleaning up...")
    for repo_dir in repo_dirs:
        cleanup_repo(repo_dir)
    
    print("\nPhase 3 completed: All directories cleaned up.")
    print("\n=== Benchmark completed ===")

if __name__ == "__main__":
    main()