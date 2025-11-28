#!/usr/bin/env python3
"""Parse forge test output and compare with baseline."""

import sys
import re
from pathlib import Path
from collections import defaultdict


def parse_forge_output(log_file):
    """Parse forge test output and extract test results."""
    results = {}

    if not log_file.exists():
        print(f"Error: Forge output log not found: {log_file}")
        sys.exit(1)

    print(f"Parsing test results from {log_file}...")

    with open(log_file, 'r', encoding='utf-8', errors='ignore') as f:
        for line in f:
            line = line.strip()

            # Match [PASS] lines
            if line.startswith('[PASS]'):
                match = re.search(r'^\[PASS\]\s+([^\s(]+)', line)
                if match:
                    results[match.group(1)] = 'PASS'

            # Match [FAIL lines
            elif line.startswith('[FAIL'):
                match = re.search(r'^\[FAIL[^\]]*\]\s+([^\s(]+)', line)
                if match:
                    results[match.group(1)] = 'FAIL'

    return results


def save_results(results, output_file):
    """Save results to file."""
    with open(output_file, 'w') as f:
        for test, status in sorted(results.items()):
            f.write(f"{test}:{status}\n")


def load_results(file_path):
    """Load results from file."""
    results = {}
    if file_path.exists():
        with open(file_path, 'r') as f:
            for line in f:
                line = line.strip()
                if ':' in line:
                    test, status = line.split(':', 1)
                    results[test] = status
    return results


def print_summary(project_name, current_results, baseline_results=None):
    """Print test results summary."""
    passing = [t for t, s in current_results.items() if s == 'PASS']
    failing = [t for t, s in current_results.items() if s == 'FAIL']

    print("━" * 60)
    print(f"Test Results for {project_name}")
    print("━" * 60)
    print(f"Total tests: {len(current_results)}")
    print(f"  ✓ Passing: {len(passing)}")
    print(f"  ✗ Failing: {len(failing)}")
    print()

    if failing:
        print("Failed tests:")
        for test in sorted(failing):
            print(f"  - {test}")
        print()

    print(f"Results saved to: test-results-{project_name}.txt")
    print()

    if baseline_results is None:
        print("No baseline file specified (first run or master branch)")

    print("━" * 60)


def compare_with_baseline(project_name, current_results, baseline_results):
    """Compare current results with baseline."""
    baseline_passing = {t for t, s in baseline_results.items() if s == 'PASS'}
    baseline_failing = {t for t, s in baseline_results.items() if s == 'FAIL'}

    current_passing = {t for t, s in current_results.items() if s == 'PASS'}
    current_failing = {t for t, s in current_results.items() if s == 'FAIL'}

    # Find regressions (passed before, failing now)
    regressions = baseline_passing & current_failing

    # Find improvements (failed before, passing now)
    improvements = baseline_failing & current_passing

    # Find new tests
    all_baseline = set(baseline_results.keys())
    all_current = set(current_results.keys())
    new_tests = all_current - all_baseline

    print("Comparing test results for", project_name)
    print("━" * 60)
    print("Test Statistics:")
    print(f"  Baseline:  {len(baseline_passing)} passing, {len(baseline_failing)} failing")
    print(f"  Current:   {len(current_passing)} passing, {len(current_failing)} failing")
    print()

    if improvements:
        print(f"Improvements: {len(improvements)} test(s) now passing")
        for test in sorted(improvements):
            print(f"  - {test}")
        print()

    if new_tests:
        print(f"New tests detected: {len(new_tests)}")
        for test in sorted(new_tests):
            status = current_results[test]
            print(f"  - {test} ({status})")
        print()

    if regressions:
        print(f"ERROR: REGRESSIONS DETECTED - {len(regressions)} test(s) now failing")
        print()
        print("The following tests passed in the baseline but are now failing:")
        for test in sorted(regressions):
            print(f"  - {test}")
        print()
        print("━" * 60)
        print("Regression check FAILED")
        return False

    print("No regressions detected")
    print("━" * 60)
    return True


def main():
    if len(sys.argv) < 3:
        print("Usage: check-test-regression.py PROJECT_NAME FORGE_OUTPUT_LOG [BASELINE_FILE]")
        sys.exit(1)

    project_name = sys.argv[1]
    log_file = Path(sys.argv[2])
    baseline_file = Path(sys.argv[3]) if len(sys.argv) > 3 else None

    # Parse current results
    current_results = parse_forge_output(log_file)

    # Check if any results were found
    if not current_results:
        print(f"WARNING: No test results found in {log_file}")
        print("This usually means tests failed to compile or run.")

    # Save current results
    output_file = f"test-results-{project_name}.txt"
    save_results(current_results, output_file)

    # If no baseline, just print summary and exit
    if not baseline_file or not baseline_file.exists():
        print_summary(project_name, current_results)
        if baseline_file and not baseline_file.exists():
            print(f"WARNING: No baseline file found at {baseline_file}")
            print("This is the first PR run - baseline will be created on master merge")
            print("━" * 60)
        sys.exit(0)

    # Load baseline and compare
    baseline_results = load_results(baseline_file)
    success = compare_with_baseline(project_name, current_results, baseline_results)

    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
