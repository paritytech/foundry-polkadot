#!/usr/bin/env python3

import sys
import json
from pathlib import Path


def parse_forge_json(json_file):
    results = {}

    if not json_file.exists():
        print(f"Error: Forge JSON output not found: {json_file}")
        sys.exit(1)

    print(f"Parsing test results from {json_file}...")

    with open(json_file, 'r', encoding='utf-8') as f:
        data = json.load(f)

    for contract_key, contract_data in data.items():
        contract_name = contract_key.split(':')[-1] if ':' in contract_key else contract_key

        test_results = contract_data.get('test_results', {})
        for test_name, test_data in test_results.items():
            status = test_data.get('status', 'Unknown')
            if status == 'Success':
                status = 'PASS'
            elif status == 'Failure':
                status = 'FAIL'
            else:
                status = 'FAIL'

            test_id = f"{contract_name}::{test_name}"
            results[test_id] = status

    return results


def save_results(results, output_file):
    with open(output_file, 'w') as f:
        json.dump(results, f, indent=2, sort_keys=True)


def load_results(file_path):
    if file_path.exists():
        with open(file_path, 'r') as f:
            return json.load(f)
    return {}


def print_summary(project_name, current_results, baseline_results=None):
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

    print(f"Results saved to: test-results-{project_name}.json")
    print()

    if baseline_results is None:
        print("No baseline file specified (first run or master branch)")

    print("━" * 60)


def compare_with_baseline(project_name, current_results, baseline_results):
    baseline_passing = {t for t, s in baseline_results.items() if s == 'PASS'}
    baseline_failing = {t for t, s in baseline_results.items() if s == 'FAIL'}

    current_passing = {t for t, s in current_results.items() if s == 'PASS'}
    current_failing = {t for t, s in current_results.items() if s == 'FAIL'}

    regressions = baseline_passing & current_failing
    improvements = baseline_failing & current_passing
    new_tests = set(current_results.keys()) - set(baseline_results.keys())

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
        print("Usage: check-test-regression.py PROJECT_NAME FORGE_JSON_OUTPUT [BASELINE_FILE]")
        sys.exit(1)

    project_name = sys.argv[1]
    json_file = Path(sys.argv[2])
    baseline_file = Path(sys.argv[3]) if len(sys.argv) > 3 else None

    current_results = parse_forge_json(json_file)

    if not current_results:
        print(f"WARNING: No test results found in {json_file}")
        print("This usually means tests failed to compile or run.")

    output_file = f"test-results-{project_name}.json"
    save_results(current_results, output_file)

    if not baseline_file or not baseline_file.exists():
        print_summary(project_name, current_results)
        if baseline_file and not baseline_file.exists():
            print(f"WARNING: No baseline file found at {baseline_file}")
            print("This is the first PR run - baseline will be created on master merge")
            print("━" * 60)
        sys.exit(0)

    baseline_results = load_results(baseline_file)
    success = compare_with_baseline(project_name, current_results, baseline_results)

    sys.exit(0 if success else 1)


if __name__ == '__main__':
    main()
