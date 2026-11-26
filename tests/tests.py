import os
import subprocess
import sys

tests_dir = "./tests"

is_release_mode = True

total_tests = 0
passed_tests = 0

ccr_additional_flags = {
    "test_optimizer.c": "-O1",
}

def has_crashed(return_code):
    return return_code < 0

def run_test(test_file):
    global total_tests, passed_tests, is_release_mode
    if is_release_mode:
        ccr_path = "target/release/ccr"
    else:
        ccr_path = "target/debug/ccr"
    total_tests += 1
    print(f"running {test_file} :", end='', flush=True)
    flags = ""
    if ccr_additional_flags.get(test_file) != None:
        flags = ccr_additional_flags[test_file]
    output_file = f"{tests_dir}/a.out"
    cmd = f"{ccr_path} {tests_dir}/{test_file} -o {output_file} {flags}"
    # TODO : also pipe stderr separately and only print it if error
    pipe = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE)
    out_stdout, _ = pipe.communicate()
    return_code = pipe.returncode

    pipe = subprocess.Popen(output_file, shell=True, stdout=subprocess.PIPE)
    out_stdout, _ = pipe.communicate()
    a_out_return_code = pipe.returncode
    if return_code == 0 and not has_crashed(a_out_return_code):
        passed_tests += 1
        print("✅")
    else:
        print("❌")
    # TODO : run the executable generated

# TODO : rebuild the compiler

# TODO : add release mode support
# TODO : add progress bar/spinner
def rebuild_compiler():
    global is_release_mode
    print("building compiler")
    cmd = "cargo build"
    if is_release_mode:
        cmd += " --release"
    pipe = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    out, _ = pipe.communicate()
    if pipe.returncode != 0:
        sys.exit(f"failed when building the compiler :\n {out.decode("utf-8", errors="replace")}")

for i in range(1, len(sys.argv)):
    arg = sys.argv[i]
    if arg == "--release":
        is_release_mode = True

rebuild_compiler()
test_files = list(filter(lambda f : f.endswith(".c"), os.listdir(path = './tests')))
for test_file in test_files:
    run_test(test_file)

print(f"{passed_tests}/{total_tests} passed")