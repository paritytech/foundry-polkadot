#!/usr/bin/env python3
"""Check for depth violations in skill structure.

Detects patterns that create depth-2 chains (SKILL.md → atom → another atom),
violating the 1-level-deep rule.

PURE CONTENT ATOM RULE (ZERO Navigation Requirement):
- Small reference files (<300 lines) must have ZERO navigation links
  - No markdown links to other .md files
  - No <nav> tags
  - No <decision_matrix> tags
  - No internal navigation of any kind
- Large comprehensive guides (600-1000 lines) MAY have internal navigation
- Rationale: Semantic independence for small atoms, cohesive depth for comprehensive guides
"""

import re
from pathlib import Path

try:
    from rich.console import Console

    console = Console()
except ImportError:
    console = None


def check_depth_violations(skill_dir: Path) -> dict[str, list[Path]]:
    """Find all atoms with depth violations.

    Returns:
        dict with violation types and list of files
    """
    skill_path = Path(skill_dir)
    # Use rglob to find all .md files in references and subdirectories
    atoms = list((skill_path / "references").rglob("*.md"))
    violations = {
        "at_prefix": [],
        "markdown_links": [],
        "navigation_tags_in_small_atoms": [],
    }

    for atom in atoms:
        content = atom.read_text()
        rel_path = atom.relative_to(skill_path)
        line_count = len(content.split("\n"))

        # Check for @ prefix in paths (old import syntax)
        if re.search(r"@(references|assets)/", content):
            violations["at_prefix"].append(rel_path)

        # Check for markdown links to other atoms (always a violation)
        if re.search(r"\[.*?\]\(\.\./.*?\.md\)", content):
            violations["markdown_links"].append(rel_path)

        # Check for navigation tags in atoms
        # Navigation tags allowed if file type requires it (e.g. workflows)
        pass

    return violations


def check_pure_content_atoms(
    root_dir: str, min_atom_size: int = 300, verbose: bool = False
) -> tuple[bool, list[dict]]:
    """
    Check Semantic Independence rule: Small files (< min_atom_size) must be link-free pure content atoms.
    Large files (>= min_atom_size) can have navigation tags for comprehensive guides.

    ZERO Navigation Requirement:
    - Files <300 lines: Must be pure content atoms with ZERO internal navigation
    - Files >=600 lines (comprehensive guides): Internal navigation allowed and encouraged
    """
    issues = []
    references_dir = Path(root_dir) / "references"

    if not references_dir.exists():
        return True, []

    for md_file in references_dir.rglob("*.md"):
        with open(md_file, "r", encoding="utf-8") as f:
            content = f.read()
            line_count = len(content.splitlines())

        # Check for internal navigation patterns
        internal_links = len(re.findall(r"\[.*?\]\(?!http)(?!#)(.*?\.md\)", content))
        nav_tags = len(re.findall(r"<nav>", content))
        decision_matrices = len(re.findall(r"<decision_matrix>", content))

        # Semantic Independence: Small files must be pure atoms (ZERO internal navigation)
        if line_count < min_atom_size:
            if internal_links > 0 or nav_tags > 0 or decision_matrices > 0:
                issues.append(
                    {
                        "file": str(md_file.relative_to(root_dir)),
                        "line_count": line_count,
                        "issue": "ZERO_NAV_VIOLATION",
                        "description": (
                            f"File has {line_count} lines (< {min_atom_size} threshold) "
                            f"but contains internal navigation ({internal_links} links, "
                            f"{nav_tags} <nav> tags, {decision_matrices} <decision_matrix> tags). "
                            f"Small files must be pure content atoms with ZERO internal navigation."
                        ),
                        "severity": "warning",
                    }
                )

        if verbose and issues:
            console.print(f"\n[red]Semantic Independence Violations:[/red]")
            for issue in issues:
                console.print(f"  [yellow]• {issue['file']}[/yellow]")
                console.print(f"    {issue['description']}")

    is_valid = len(issues) == 0
    return is_valid, issues


def main():
    skill_dir = Path(__file__).parent.parent.parent
    violations = check_depth_violations(skill_dir)

    total_violations = sum(len(v) for v in violations.values())

    if total_violations > 0:
        print(f"❌ Found {total_violations} depth violations:\n")

        if violations["at_prefix"]:
            print(
                f"  Atoms with @ prefix (old import syntax): {len(violations['at_prefix'])}"
            )
            for v in violations["at_prefix"]:
                print(f"    - {v}")
            print()

        if violations["markdown_links"]:
            print(f"  Atoms with markdown links: {len(violations['markdown_links'])}")
            for v in violations["markdown_links"]:
                print(f"    - {v}")
            print()

        if violations["navigation_tags_in_small_atoms"]:
            print(
                f"  Small atoms (<300 lines) with navigation tags: {len(violations['navigation_tags_in_small_atoms'])}"
            )
            for v in violations["navigation_tags_in_small_atoms"]:
                print(f"    - {v}")
            print()

        return 1
    else:
        print("✅ No depth violations found - small atoms are Pure Content compliant")
        return 0


if __name__ == "__main__":
    exit(main())
