#!/usr/bin/env python3
"""
Broken Reference Detector for OpenCode Skills

Scans SKILL.md files for file references that point to non-existent files.
Helps maintain hub & spoke integrity by detecting orphaned or broken references.
"""

from pathlib import Path
import argparse
import sys
import re


class BrokenReferenceDetector:
    """Detects broken file references in SKILL.md files."""

    def __init__(self, skill_path: Path):
        self.skill_path = skill_path
        self.broken_refs = []
        self.valid_refs = []

    def detect(self) -> dict:
        """Scan for broken references."""
        skill_md = self.skill_path / "SKILL.md"

        if not skill_md.exists():
            return {"error": "SKILL.md not found"}

        content = skill_md.read_text()

        # Strip code blocks to avoid false positives in examples
        content_no_code = re.sub(r"```.*?```", "", content, flags=re.DOTALL)
        content_no_code = re.sub(r"`.*?`", "", content_no_code)

        old_style_refs = re.findall(r"@references/([^\s\)\`\"<]+)", content_no_code)
        new_style_refs = re.findall(r"<navigate_to>([^<]+)</navigate_to>", content_no_code)
        external_doc_refs = re.findall(
            r'<external_doc\s+system="([^"]+)"\s+libraryId="([^"]+)"\s+query="([^"]+)"\s*/?>',
            content_no_code
        )
        source_inspection_refs = re.findall(
            r'<source_inspection\s+script="([^"]+)"(?:\s+args="([^"]*)")?\s*/?>',
            content_no_code
        )

        def strip_references_prefix(ref: str) -> str:
            ref = ref.strip()
            if ref.startswith("references/"):
                return ref[len("references/") :]
            return ref

        all_refs = [strip_references_prefix(ref) for ref in old_style_refs]
        all_refs.extend(strip_references_prefix(ref) for ref in new_style_refs)

        unique_refs = list(dict.fromkeys(all_refs))

        if not unique_refs:
            return {"status": "no_imports", "message": "No file references found"}

        for import_path in unique_refs:
            full_path = self.skill_path / "references" / import_path

            if full_path.exists():
                self.valid_refs.append(import_path)
            else:
                self.broken_refs.append(import_path)

        return {
            "total": len(unique_refs),
            "valid": len(self.valid_refs),
            "broken": len(self.broken_refs),
            "broken_refs": self.broken_refs,
            "valid_refs": self.valid_refs,
            "external_doc_refs": external_doc_refs,
            "source_inspection_refs": source_inspection_refs,
        }

    def validate_external_doc_refs(self, refs: list) -> list:
        """Validate external documentation references."""
        warnings = []
        valid_systems = ['context7', 'github', 'official-docs', 'pkg-manager']
        
        for system, library_id, query in refs:
            # Validate system attribute
            if system not in valid_systems:
                warnings.append(f"Unknown documentation system '{system}'")
            
            # Validate libraryId format based on system
            if system == 'context7' and not library_id.startswith('/'):
                warnings.append(f"Context7 libraryId should start with '/': {library_id}")
            
            # Validate query length
            if len(query.strip()) < 5:
                warnings.append(f"Query too short: {query}")
        
        return warnings

    def validate_source_inspection_refs(self, refs: list, project_root: Path) -> list:
        """Validate source inspection script references."""
        warnings = []
        
        for script_path, args in refs:
            # Validate script path format
            if not script_path.startswith('.claude/skills/') and not script_path.startswith('.'):
                warnings.append(f"Script path should be relative to project root: {script_path}")
            
            # Validate script exists
            full_path = project_root / script_path
            if not full_path.exists():
                warnings.append(f"Script file not found: {script_path}")
            elif not full_path.is_file():
                warnings.append(f"Script path is not a file: {script_path}")
            else:
                # Check if executable (has shebang)
                try:
                    first_line = full_path.read_text().split('\n')[0]
                    if not first_line.startswith('#!'):
                        warnings.append(f"Script missing shebang (not executable): {script_path}")
                except Exception:
                    warnings.append(f"Cannot read script file: {script_path}")
        
        return warnings

    def print_report(self):
        """Print formatted report."""
        result = self.detect()

        if "error" in result:
            print(f"❌ Error: {result['error']}")
            return 1

        if result.get("status") == "no_imports":
            print("⚠️  No file references found in SKILL.md")
            return 0

        print(f"\n{'=' * 70}")
        print(f"🔍 Broken Reference Detector: {self.skill_path.name}")
        print(f"{'=' * 70}\n")

        print(f"📊 Summary:")
        print(f"   Total file references: {result['total']}")
        print(f"   ✅ Valid: {result['valid']}")
        print(f"   ❌ Broken: {result['broken']}")

        # External documentation references
        if result.get("external_doc_refs"):
            print(f"\n📚 External Documentation References ({len(result['external_doc_refs'])}):")
            for system, lib_id, query in result["external_doc_refs"]:
                print(f"   • {system}: {lib_id}")
                print(f"     Query: {query}")
            
            # Validate external doc references
            warnings = self.validate_external_doc_refs(result["external_doc_refs"])
            if warnings:
                print(f"\n⚠️  External Doc Validation Warnings:")
                for warning in warnings:
                    print(f"   • {warning}")

        # Source inspection references
        if result.get("source_inspection_refs"):
            print(f"\n🔬 Source Inspection References ({len(result['source_inspection_refs'])}):")
            for script_path, args in result["source_inspection_refs"]:
                print(f"   • Script: {script_path}")
                if args:
                    print(f"     Args: {args}")
            
            # Validate source inspection references
            project_root = self.skill_path.parent.parent.parent  # Navigate from .claude/skills/skill-name/ to project root
            warnings = self.validate_source_inspection_refs(result["source_inspection_refs"], project_root)
            if warnings:
                print(f"\n⚠️  Source Inspection Validation Warnings:")
                for warning in warnings:
                    print(f"   • {warning}")

        if result["broken"] > 0:
            print(f"\n❌ Broken References ({result['broken']}):")
            for ref in result["broken_refs"]:
                expected_path = self.skill_path / "references" / ref
                print(f"   • @{ref}")
                print(f"     Expected: {expected_path}")
                print(f"     Status: File not found")

        if result["valid"] > 0:
            print(f"\n✅ Valid References ({result['valid']}):")
            for ref in result["valid_refs"]:
                print(f"   • @{ref}")

        print(f"\n{'=' * 70}\n")

        if result["broken"] == 0:
            print("✅ All references valid! Hub & spoke integrity maintained.")
            return 0
        else:
            print(f"❌ Found {result['broken']} broken reference(s). Fix required!")
            return 1


def main():
    parser = argparse.ArgumentParser(
        description="Detect broken file references in OpenCode skills",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Check skill-creator
  python check_broken_references.py .claude/skills/skill-creator

  # Check with verbose output
  python check_broken_references.py .claude/skills/skill-creator --verbose

  # Fix mode (suggest corrections)
  python check_broken_references.py .claude/skills/skill-creator --fix
        """,
    )

    parser.add_argument("skill_path", type=str, help="Path to the skill directory")

    parser.add_argument(
        "-v", "--verbose", action="store_true", help="Enable verbose output"
    )

    parser.add_argument(
        "--fix", action="store_true", help="Suggest corrections for broken references"
    )

    args = parser.parse_args()

    skill_path = Path(args.skill_path)

    if not skill_path.exists():
        print(f"❌ Error: Skill path '{skill_path}' does not exist")
        sys.exit(1)

    detector = BrokenReferenceDetector(skill_path)
    return detector.print_report()


if __name__ == "__main__":
    sys.exit(main())
