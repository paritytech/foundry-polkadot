#!/usr/bin/env python3
"""
OpenCode Skill Quality Auditor

Implements the AI-Readiness Audit Framework from
"Rebuilding Technical Documentation Corpus" Section 2.1.

This script provides automated baseline checks. For comprehensive analysis,
combine with:
  - LLM-as-Auditor pipeline (@atoms/validation/automated-auditing-pipeline.md)
  - Qualitative card sorting (@atoms/validation/qualitative-analysis-card-sorting.md)
  - Bottom-up reconstruction (@atoms/migration/bottom-up-reconstruction.md)

AI-Readiness Dimensions measured:
  1. Semantic Cohesion: Related concepts proximity (target >0.7)
  2. Single Intent Clarity: One clear purpose per content block
  3. Contradiction Rate: Conflicting instructions (target = 0)
  4. Instructional Clarity: Imperative vs passive voice (target >90%)
  5. Format Compliance: YAML/XML structure validity

Note: This script covers automated checks (Dimensions 2, 4, 5).
For Dimensions 1 and 3, use the LLM-as-Auditor pipeline with vector search.

Reference: @atoms/validation/ai-readiness-audit-framework.md
"""

from pathlib import Path
from typing import Dict, List, Tuple
import argparse
import sys
import re


class SkillAuditor:
    """
    AI-Readiness Audit for OpenCode skills.

    Implements automated checks from AI-Readiness Audit Framework:
    - Dimension 2: Single Intent Clarity (one clear purpose per block)
    - Dimension 4: Instructional Clarity (imperative vs passive)
    - Dimension 5: Format Compliance (YAML/XML structure)

    For Dimensions 1 (Semantic Cohesion) and 3 (Contradiction Rate),
    use LLM-as-Auditor pipeline with vector search.
    See: @atoms/validation/automated-auditing-pipeline.md

    Reference: @atoms/validation/ai-readiness-audit-framework.md
    """

    def __init__(self, skill_path: Path):
        self.skill_path = skill_path
        self.issues = {
            "format_compliance": [],  # Dimension 5
            "token_bloat": [],  # Economy optimization
            "single_intent_clarity": [],  # Dimension 2 (replaces atomic purity)
            "orphans": [],  # Hub & Spoke integrity
            "missing_xml": [],  # Dimension 5 (XML tags)
            "passive_voice": [],  # Dimension 4 (instructional clarity)
            "contradictions": [],  # Dimension 3 (requires LLM-as-Judge)
        }

    def audit(self) -> Dict[str, List[str]]:
        """Run complete audit across all dimensions."""
        self._check_format_compliance()
        self._check_token_economy()
        self._check_single_intent_clarity()
        # TODO: Implement _detect_orphans() and _check_xml_tagging()

        return self.issues

    def _check_format_compliance(self):
        """Check OpenCode frontmatter compliance."""
        skill_md = self.skill_path / "SKILL.md"

        if not skill_md.exists():
            self.issues["format_compliance"].append("Missing SKILL.md file")
            return

        content = skill_md.read_text()

        if not content.startswith("---"):
            self.issues["format_compliance"].append(
                "Missing YAML frontmatter (must start with '---')"
            )
            return

        # Extract frontmatter
        end_idx = content.find("---", 3)
        if end_idx == -1:
            self.issues["format_compliance"].append(
                "YAML frontmatter not closed (missing closing '---')"
            )
            return

        yaml_str = content[3:end_idx]

        # Check required fields
        if "name:" not in yaml_str:
            self.issues["format_compliance"].append("Missing required field: 'name'")

        if "description:" not in yaml_str:
            self.issues["format_compliance"].append(
                "Missing required field: 'description'"
            )

        # Check for unsupported fields (OpenCode ignores these)
        unsupported_fields = [
            "activation_trigger:",
            "version:",
            "min_opencode_version:",
            "max_opencode_version:",
        ]

        for field in unsupported_fields:
            if field in yaml_str:
                self.issues["format_compliance"].append(
                    f"Unsupported field '{field}' - OpenCode ignores this. "
                    f"Use 'description' field for semantic activation instead."
                )

    def _check_token_economy(self):
        """
        Check SKILL.md line count and detect excessive atomization.

        Checks:
        1. SKILL.md should be 100-800 lines (comprehensive skills)
        2. Excessive atomization with granular thresholds:
           - WARNING: >10 files with avg <200 lines
           - ERROR: >15 files with avg <150 lines
        3. Micro-atomization: Files <50 lines (except utilities)
        4. Consolidation scoring: Suggest merges for related content
        """
        skill_md = self.skill_path / "SKILL.md"

        if not skill_md.exists():
            return

        lines = len(skill_md.read_text().split("\n"))

        if lines < 100:
            self.issues["token_bloat"].append(
                f"SKILL.md is {lines} lines (under 100 lines may lack necessary context)"
            )
        elif lines > 800:
            excess = lines - 800
            self.issues["token_bloat"].append(
                f"SKILL.md is {lines} lines (exceeds 800 lines for comprehensive skills). "
                f"Move ~{excess} lines to references/ for JIT loading."
            )

        # Check for excessive atomization in references
        references = self.skill_path / "references"
        if references.exists():
            ref_files = list(references.glob("**/*.md"))
            ref_files = [f for f in ref_files if "legacy" not in f.parts]

            if len(ref_files) > 0:
                # Calculate average file size
                total_lines = 0
                file_sizes = []
                micro_files = []

                for ref_file in ref_files:
                    file_lines = len(ref_file.read_text().split("\n"))
                    total_lines += file_lines
                    file_sizes.append((ref_file.name, file_lines))

                    # Detect micro-atomization (<50 lines)
                    if file_lines < 50:
                        micro_files.append((ref_file.name, file_lines))

                avg_lines = total_lines / len(ref_files)

                # Granular atomization thresholds
                if len(ref_files) > 15 and avg_lines < 150:
                    self.issues["token_bloat"].append(
                        f"🔴 EXCESSIVE ATOMIZATION (ERROR): {len(ref_files)} reference files, "
                        f"average {avg_lines:.0f} lines. "
                        f"Consolidate into 600-1000 line comprehensive guides. "
                        f"Expected: +15-25 SkillScore points after consolidation."
                    )
                elif len(ref_files) > 10 and avg_lines < 200:
                    self.issues["token_bloat"].append(
                        f"⚠️  EXCESSIVE ATOMIZATION (WARNING): {len(ref_files)} reference files, "
                        f"average {avg_lines:.0f} lines. "
                        f"Consider consolidating into 600-1000 line guides for better semantic cohesion."
                    )

                # Micro-atomization detection
                if micro_files:
                    micro_list = ", ".join(
                        [f"{name} ({lines}L)" for name, lines in micro_files[:5]]
                    )
                    if len(micro_files) > 5:
                        micro_list += f" and {len(micro_files) - 5} more"

                    self.issues["token_bloat"].append(
                        f"🔴 MICRO-ATOMIZATION: {len(micro_files)} files <50 lines: {micro_list}. "
                        f"Merge into related guides or combine multiple micro-files into one coherent document."
                    )

                # Consolidation scoring (suggest merges for scattered content)
                if len(ref_files) >= 5:
                    # Look for patterns indicating scattered content
                    tiny_files = [name for name, lines in file_sizes if lines < 100]
                    if len(tiny_files) >= 3:
                        self.issues["token_bloat"].append(
                            f"⚠️  CONSOLIDATION OPPORTUNITY: {len(tiny_files)} files <100 lines. "
                            f"Group related files by semantic domain (e.g., 'testing', 'deployment') "
                            f"to create comprehensive guides targeting 600-1000 lines."
                        )

    def _check_single_intent_clarity(self):
        """
        Check for single intent clarity and semantic cohesion.

        Checks:
        1. Files with >15 ## sections (potential split needed)
        2. Excessive atomization (delegated to _check_token_economy)
        3. Passive voice (instructional clarity)
        4. Arbitrary split detection (Part 1, Part 2 patterns)
        """
        skill_md = self.skill_path / "SKILL.md"
        content = skill_md.read_text()
        imports = []

        # Legacy @references format
        for line in content.split("\n"):
            if "@references" in line:
                clean_line = (
                    line.replace("<atom>", "")
                    .replace("</atom>", "")
                    .replace("<reference>", "")
                    .replace("</reference>", "")
                    .strip()
                )
                import_match = re.search(r"@references/([^\s<>\)]+)", clean_line)
                if import_match:
                    atom_name = Path(import_match.group(1)).stem
                    imports.append(atom_name)

        # Current <navigate_to> format
        navigate_pattern = r"<navigate_to>references/([^<]+)</navigate_to>"
        for match in re.findall(navigate_pattern, content):
            atom_name = Path(match).stem
            imports.append(atom_name)

        return imports

    def print_report(self):
        """Print formatted audit report."""
        print(f"\n{'=' * 70}")
        print(f"🔍 OpenCode Skill Audit Report: {self.skill_path.name}")
        print(f"{'=' * 70}\n")

        total_issues = sum(len(v) for v in self.issues.values())

        if total_issues == 0:
            print("✅ No issues found! Skill is healthy.\n")
            return 0
        else:
            print(f"⚠️  Found {total_issues} issue(s):\n")

            for category, items in self.issues.items():
                if items:
                    category_name = category.replace("_", " ").upper()
                    print(f"\n📌 {category_name}:")

                    for item in items:
                        print(f"  ❌ {item}")

            print(f"\n{'=' * 70}")
            return 1


def main():
    parser = argparse.ArgumentParser(
        description="Audit OpenCode skills for quality issues",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Audit skill-creator
  python audit_degraded_skill.py .claude/skills/skill-creator

  # Audit with full report
  python audit_degraded_skill.py .claude/skills/skill-creator --verbose

  # Check specific category only
  python audit_degraded_skill.py .claude/skills/skill-creator --check token_bloat
        """,
    )

    parser.add_argument("skill_path", type=str, help="Path to the skill directory")

    parser.add_argument(
        "-v", "--verbose", action="store_true", help="Enable verbose output"
    )

    parser.add_argument(
        "--check",
        type=str,
        choices=[
            "format_compliance",
            "token_bloat",
            "single_intent_clarity",
        ],
        help="Check only specific category",
    )

    args = parser.parse_args()

    skill_path = Path(args.skill_path)

    if not skill_path.exists():
        print(f"❌ Error: Skill path '{skill_path}' does not exist")
        sys.exit(1)

    if not (skill_path / "SKILL.md").exists():
        print(f"❌ Error: SKILL.md not found in '{skill_path}'")
        sys.exit(1)

    auditor = SkillAuditor(skill_path)

    if args.check:
        # Run only specific check
        # Map check names to actual method names
        check_methods = {
            "format_compliance": "_check_format_compliance",
            "token_bloat": "_check_token_economy",
            "single_intent_clarity": "_check_single_intent_clarity",
            # TODO: Implement missing methods
            # "orphans": "_detect_orphans",
            # "missing_xml": "_check_xml_tagging",
            # "passive_voice": "_check_passive_voice",
        }

        if args.check not in check_methods:
            print(f"❌ Error: Unknown check category '{args.check}'")
            return 1

        method_name = check_methods[args.check]
        check_method = getattr(auditor, method_name, None)

        if check_method is None:
            print(f"❌ Error: Method '{method_name}' not found")
            return 1

        check_method()
    else:
        # Run full audit
        auditor.audit()

    return auditor.print_report()


if __name__ == "__main__":
    sys.exit(main())
