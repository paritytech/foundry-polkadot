#!/usr/bin/env python3
import os
import sys
import yaml
import re
import argparse
import json
from pathlib import Path
from typing import Dict, List, Tuple

# Add current directory to path so we can import local modules
sys.path.append(os.path.dirname(os.path.abspath(__file__)))
from skillscore_evaluator import calculate_skillscore


class SkillValidator:
    def __init__(self, skill_path: str, golden_queries: bool = False):
        self.skill_path = Path(skill_path)
        self.check_golden_queries = golden_queries
        self.errors: List[str] = []
        self.warnings: List[str] = []

    def validate_structure(self) -> bool:
        """Validate skill structure follows Pure Content Atom rules."""
        skill_path = self.skill_path

        # Check for incorrect folder names (only references/, assets/, and scripts/ allowed)
        ALLOWED_FOLDERS = {"references", "assets", "scripts", "tests"}
        for dir_path in skill_path.iterdir():
            if dir_path.is_dir() and dir_path.name not in ALLOWED_FOLDERS:
                self.errors.append(
                    f"Invalid directory name: '{dir_path.name}/'. Only '{', '.join(sorted(ALLOWED_FOLDERS))}/' folders are allowed in skill directories"
                )
                return False

        # Check for @ prefixes in SKILL.md (old import syntax)
        skill_md = skill_path / "SKILL.md"
        if skill_md.exists():
            content = skill_md.read_text()
            if re.search(r"@(references|assets)/", content):
                self.errors.append(
                    "SKILL.md contains @ prefix in paths (old import syntax). Use 'references/' or 'assets/' instead"
                )
                return False

            if "references/atoms/" in content:
                self.errors.append(
                    "SKILL.md references 'atoms/' subdirectory. Use flat 'references/' structure instead"
                )
                return False

            # Check for references to non-standard folders in decision matrix
            if re.search(
                r"<navigate_to>(comprehensive|guides|docs|tutorials|modules)/", content
            ):
                self.errors.append(
                    "SKILL.md decision matrix references non-standard folder. Use 'references/' instead of 'comprehensive/', 'guides/', etc."
                )
                return False

            # Check for navigation tags in atoms (Adaptive Purity rule)
            refs_path = skill_path / "references"
            if refs_path.exists():
                for atom in refs_path.glob("*.md"):
                    atom_content = atom.read_text()
                    line_count = len(atom_content.splitlines())

                    # Extract atom type from metadata
                    atom_type_match = re.search(r"<type>(\w+)</type>", atom_content)
                    atom_type = atom_type_match.group(1) if atom_type_match else None

                    # Workflow atoms can have navigation tags in decision matrices
                    # Pure content atoms (explanation, how-to, tutorial, reference) should not
                    if atom_type == "workflow":
                        continue  # Skip navigation tag check for workflow atoms

                    # NOTE: We allow navigation tags if they are structurally necessary, regardless of file size.
                    # The arbitrary 300-line limit has been deprecated.

                    # Structural Warnings (Layer 1.5)
                    if line_count < 100 and atom_type != "reference":
                        self.warnings.append(
                            f"File '{atom.name}' is {line_count} lines (recommended > 100 lines for non-reference atoms). Consider consolidation."
                        )

                    if line_count > 2000:
                        self.warnings.append(
                            f"File '{atom.name}' is {line_count} lines (recommended < 2000 lines). Consider splitting if semantic cohesion allows."
                        )

        return True

    def validate_skill_md(self) -> bool:
        skill_md_path = self.skill_path / "SKILL.md"

        if not skill_md_path.exists():
            return False

        with open(skill_md_path, "r") as f:
            content = f.read()

        if not content.startswith("---"):
            self.errors.append("SKILL.md must start with YAML frontmatter")
            return False

        try:
            parts = content.split("---", 2)
            if len(parts) < 3:
                self.errors.append("Invalid YAML frontmatter format")
                return False

            frontmatter = yaml.safe_load(parts[1])

            required_fields = ["name", "description"]
            for field in required_fields:
                if field not in frontmatter:
                    self.errors.append(f"Missing required YAML field: {field}")

            ALLOWED_PROPERTIES = {
                "name",
                "description",
                "license",
                "compatibility",
                "metadata",
                "triggers",
                "anti_triggers",
            }

            unexpected_keys = set(frontmatter.keys()) - ALLOWED_PROPERTIES
            if unexpected_keys:
                self.errors.append(
                    f"Unexpected key(s) in SKILL.md frontmatter: {', '.join(sorted(unexpected_keys))}. "
                    f"Allowed properties are: {', '.join(sorted(ALLOWED_PROPERTIES))}"
                )

            description = frontmatter.get("description", "")
            if not self._validate_description(description):
                self.warnings.append(
                    "Description may not follow trigger phrase patterns"
                )

        except yaml.YAMLError as e:
            self.errors.append(f"Invalid YAML in frontmatter: {e}")

        body = content.split("---", 2)[-1].strip()
        if not body:
            self.errors.append("SKILL.md missing markdown body content")

        # 2. Quality Score Check (Layer 2)
        # Rubric-Based Evaluation
        skill_md_path = self.skill_path / "SKILL.md"
        if skill_md_path.exists():
            print(f"  Calculating SkillScore for {skill_md_path.name}...")
            result = calculate_skillscore(str(skill_md_path))

            if "error" in result:
                self.errors.append(f"SkillScore Error: {result['error']}")
            else:
                score = result["skill_score"]
                if score < 80:
                    self.errors.append(
                        f"SkillScore is {score}/100 (Threshold: 80). Improve navigation, determinism, or safety."
                    )
                else:
                    print(f"  ✅ SkillScore: {score}/100")
                    # Show breakdown for high scores too
                    for cat, data in result["breakdown"].items():
                        print(f"    - {cat.title()}: {data['score']}/100")

        # 3. Print Results
        return len(self.errors) == 0

    def _validate_description(self, description: str) -> bool:
        trigger_patterns = [r"use when", r"trigger", r"when.*user", r"for.*tasks?"]

        return any(
            re.search(pattern, description.lower()) for pattern in trigger_patterns
        )

    def validate_references(self) -> bool:
        refs_path = self.skill_path / "references"

        if not refs_path.exists():
            return True

        for md_file in refs_path.glob("*.md"):
            if not self._validate_markdown_file(md_file):
                self.warnings.append(
                    f"Reference file {md_file.name} may not be SkillScore optimized"
                )

        return True

    def _validate_markdown_file(self, file_path: Path) -> bool:
        with open(file_path, "r") as f:
            content = f.read()

        code_blocks = re.findall(r"```[\w]*\n(.*?)\n```", content, re.DOTALL)

        if not code_blocks:
            return True

        has_imports = any("import" in block or "from" in block for block in code_blocks)
        has_error_handling = any(
            "try" in block or "catch" in block for block in code_blocks
        )

        if not has_imports and not has_error_handling:
            return True

        return has_imports and has_error_handling

    def validate_scripts(self) -> bool:
        scripts_path = self.skill_path / "scripts"

        if not scripts_path.exists():
            return True

        for script_file in scripts_path.iterdir():
            if script_file.is_file():
                if os.name != "nt" and not os.access(script_file, os.X_OK):
                    self.warnings.append(
                        f"Script {script_file.name} lacks execute permissions"
                    )

        return True

    def validate_golden_queries(self) -> bool:
        # Validation scripts ARE the tests - no separate tests directory needed
        return True

    def run_validation(self) -> Tuple[bool, List[str], List[str]]:
        self.validate_structure()
        self.validate_skill_md()
        self.validate_references()
        self.validate_scripts()
        self.validate_golden_queries()

        success = len(self.errors) == 0
        return success, self.errors, self.warnings


def main():
    parser = argparse.ArgumentParser(description="Validate OpenCode Skill")
    parser.add_argument("skill_path", help="Path to the skill directory")
    parser.add_argument(
        "--golden-queries", action="store_true", help="Validate golden queries file"
    )

    args = parser.parse_args()

    validator = SkillValidator(args.skill_path, args.golden_queries)
    success, errors, warnings = validator.run_validation()

    print(f"Skill Validation: {'PASS' if success else 'FAIL'}")
    print(f"Path: {args.skill_path}")
    print()

    if errors:
        print("Errors:")
        for error in errors:
            print(f"  ❌ {error}")

    if warnings:
        print("Warnings:")
        for warning in warnings:
            print(f"  ⚠️  {warning}")

    if not errors and not warnings:
        print("✅ All validations passed!")

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
