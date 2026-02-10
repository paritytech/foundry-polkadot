#!/usr/bin/env python3
"""Unified validation CLI for OpenCode skills.

Checks:
  - Broken reference links (@references/ and <navigate_to>)
  - Empty reference files
  - Deprecated navigation syntax
  - Depth violations (markdown links between references)
"""

import sys
import re
from pathlib import Path
from typing import Dict, List


def strip_code_blocks(content: str) -> str:
    """Remove markdown code blocks to avoid false positives."""
    content = re.sub(r"```.*?```", "", content, flags=re.DOTALL)
    content = re.sub(r"`[^`]+`", "", content)
    return content


def extract_references(content: str) -> Dict[str, List[str]]:
    """Extract all references from content."""
    content_no_code = strip_code_blocks(content)

    old_style = re.findall(r"@references/([^\s\)\`\"<]+)", content_no_code)
    new_style = re.findall(r"<navigate_to>([^<]+)</navigate_to>", content_no_code)

    return {"old_style": old_style, "new_style": new_style}


def check_file_exists(skill_root: Path, ref_path: str) -> bool:
    """Check if a reference file exists."""
    normalized = ref_path.strip()
    if not normalized.startswith("references/"):
        normalized = f"references/{normalized}"
    return (skill_root / normalized).exists()


def check_broken_references(skill_root: Path) -> Dict:
    """Check for broken reference links."""
    result = {
        "check": "broken_references",
        "passed": True,
        "errors": [],
        "warnings": [],
        "info": [],
    }
    skill_md = skill_root / "SKILL.md"

    if not skill_md.exists():
        result["errors"].append("SKILL.md not found")
        result["passed"] = False
        return result

    content = skill_md.read_text()
    refs = extract_references(content)

    for ref in refs["old_style"]:
        if not check_file_exists(skill_root, ref):
            result["errors"].append(f"Broken reference: @{ref}")
            result["passed"] = False

    for ref in refs["new_style"]:
        if not check_file_exists(skill_root, ref):
            result["errors"].append(
                f"Broken reference: <navigate_to>{ref}</navigate_to>"
            )
            result["passed"] = False

    if result["passed"]:
        result["info"].append(
            f"Checked {len(refs['old_style']) + len(refs['new_style'])} references"
        )

    return result


def check_empty_files(skill_root: Path) -> Dict:
    """Check for empty reference files."""
    result = {
        "check": "empty_files",
        "passed": True,
        "errors": [],
        "warnings": [],
        "info": [],
    }
    refs_dir = skill_root / "references"

    if not refs_dir.exists():
        result["info"].append("No references directory found")
        return result

    empty_files = []
    for ref_file in refs_dir.rglob("*.md"):
        if sum(1 for line in ref_file.read_text().splitlines() if line.strip()) == 0:
            empty_files.append(ref_file.relative_to(skill_root))

    for empty_file in empty_files:
        result["errors"].append(f"Empty file: {empty_file}")
        result["passed"] = False

    if result["passed"]:
        result["info"].append("All reference files have content")

    return result


def check_navigation_format(skill_root: Path) -> Dict:
    """Check for deprecated navigation syntax."""
    result = {
        "check": "navigation_format",
        "passed": True,
        "errors": [],
        "warnings": [],
        "info": [],
    }
    skill_md = skill_root / "SKILL.md"

    if not skill_md.exists():
        result["errors"].append("SKILL.md not found")
        result["passed"] = False
        return result

    content = skill_md.read_text()

    deprecated_refs = re.findall(r"@references/[^\s\)]+", content)

    for deprecated in deprecated_refs:
        result["warnings"].append(
            f"Deprecated syntax: {deprecated} (use <navigate_to>)"
        )

    if not deprecated_refs:
        result["info"].append("Navigation format is correct")

    return result


def check_depth_violations(skill_root: Path) -> Dict:
    """Check for excessive nesting depth."""
    result = {
        "check": "depth_violations",
        "passed": True,
        "errors": [],
        "warnings": [],
        "info": [],
    }

    refs_dir = skill_root / "references"
    if not refs_dir.exists():
        return result

    for ref_file in refs_dir.rglob("*.md"):
        content = ref_file.read_text()

        links = re.findall(r"\[([^\]]+)\]\(@references/([^\)]+)\)", content)

        for title, target in links:
            result["errors"].append(
                f"{ref_file.relative_to(skill_root)}: "
                f"Markdown link to @{target} (use <navigate_to>)"
            )
            result["passed"] = False

    if result["passed"]:
        result["info"].append("No depth violations found")

    return result


def main():
    if len(sys.argv) < 2:
        print("Usage: validate.py <skill-path>")
        sys.exit(1)

    skill_path = Path(sys.argv[1])

    checks = [
        check_broken_references(skill_path),
        check_empty_files(skill_path),
        check_navigation_format(skill_path),
        check_depth_violations(skill_path),
    ]

    total_passed = sum(1 for c in checks if c["passed"])
    total_checks = len(checks)

    if total_passed == total_checks:
        print(f"✅ All checks passed ({total_checks}/{total_checks})")
    else:
        print(f"❌ {total_checks - total_passed}/{total_checks} checks failed")
        print()

        for result in checks:
            if result["passed"]:
                continue

            print(f"❌ {result['check']}")
            for error in result["errors"]:
                print(f"  ERROR: {error}")
            for warning in result["warnings"]:
                print(f"  WARNING: {warning}")
            print()

    sys.exit(0 if total_passed == total_checks else 1)


if __name__ == "__main__":
    main()
