#!/usr/bin/env python3
"""
Skill Generation Script
Automates the creation of Claude Skill directory structure
"""

import os
import sys
import shutil
from pathlib import Path
from typing import Dict, Any


class SkillGenerator:
    def __init__(self, skill_name: str, skill_path: str = None):
        self.skill_name = skill_name
        self.base_path = Path(skill_path) if skill_path else Path('.claude/skills')
        self.skill_path = self.base_path / skill_name

    def create_directory_structure(self) -> None:
        """Create canonical skill directory structure"""
        dirs = ["scripts", "assets", "references"]

        # Create main skill directory
        self.skill_path.mkdir(parents=True, exist_ok=True)

        # Create subdirectories
        for dir_name in dirs:
            (self.skill_path / dir_name).mkdir(exist_ok=True)

    def generate_skill_md(self, description: str, workflow: str = None) -> None:
        """Generate SKILL.md with proper structure"""
        frontmatter = f"""---
name: {self.skill_name}
description: {description}
---

# {self.skill_name.replace("-", " ").title()}

## Context
You are an expert in [domain]. Your goal is to [primary objective].

## Workflow

1. **Requirement Analysis**: Identify user needs and constraints
2. **Planning**: Create detailed execution plan
3. **Implementation**: Execute the planned steps
4. **Validation**: Verify results meet requirements
5. **Optimization**: Refine based on feedback

## References
For detailed guidance, read references/best-practices.md
For examples, read references/examples.md

## Output Format
Always provide results in a structured, actionable format."""

        skill_md_path = self.skill_path / "SKILL.md"
        with open(skill_md_path, "w") as f:
            f.write(frontmatter)

    def create_template_files(self) -> None:
        """Create template files in assets and references"""

        # Create basic README in references
        readme_content = f"""# {self.skill_name} Documentation

## Overview
[Describe what this skill does and when to use it]

## API Reference
[Document any APIs, functions, or interfaces]

## Examples
[Provide complete, runnable examples]

## Best Practices
[Include optimization tips and common patterns]

## Error Handling
[Document error conditions and recovery strategies]
"""

        readme_path = self.skill_path / "references" / "README.md"
        with open(readme_path, "w") as f:
            f.write(readme_content)

        # Create example asset template
        asset_template = """# Example Template

This is a template file that can be copied and customized.

## Usage
[Instructions for using this template]
"""

        template_path = self.skill_path / "assets" / "template.md"
        with open(template_path, "w") as f:
            f.write(asset_template)

    def create_validation_script(self) -> None:
        """Create a basic validation script"""
        validation_script = """#!/bin/bash
# Basic skill validation script

echo "Validating skill structure..."

# Check for required files
if [ ! -f "SKILL.md" ]; then
    echo "ERROR: Missing SKILL.md"
    exit 1
fi

# Check YAML frontmatter
if ! head -1 SKILL.md | grep -q "^---$"; then
    echo "ERROR: SKILL.md missing YAML frontmatter"
    exit 1
fi

echo "✅ Skill structure validation passed"
"""

        script_path = self.skill_path / "scripts" / "validate.sh"
        with open(script_path, "w") as f:
            f.write(validation_script)

        # Make script executable
        script_path.chmod(0o755)

    def generate_skill(self, description: str) -> bool:
        """Generate complete skill structure"""
        try:
            self.create_directory_structure()
            self.generate_skill_md(description)
            self.create_template_files()
            self.create_validation_script()

            print(
                f"✅ Skill '{self.skill_name}' created successfully at {self.skill_path}"
            )
            return True

        except Exception as e:
            print(f"❌ Failed to create skill: {e}")
            return False


def main():
    if len(sys.argv) < 3:
        print("Usage: python generate_skill.py <skill_name> <description>")
        print(
            "Example: python generate_skill.py data-processor 'Processes and validates data files'"
        )
        sys.exit(1)

    skill_name = sys.argv[1]
    description = sys.argv[2]

    generator = SkillGenerator(skill_name)
    success = generator.generate_skill(description)

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
