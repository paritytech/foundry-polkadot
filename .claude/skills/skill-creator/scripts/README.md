# Skill Creator Scripts

## Quick Start

```bash
# Run all validation checks
python3 lib/validate.py .claude/skills/skill-creator

# Calculate SkillScore (Tier 1: Fast static analysis)
python3 skillscore_evaluator.py .claude/skills/my-skill/SKILL.md

# Run with JSON output (for CI/CD)
python3 lib/validate.py .claude/skills/skill-creator --format=json

# Run with GitLab CI output
python3 lib/validate.py .claude/skills/skill-creator --format=gitlab
```

## SkillScore: Two-Tiered Quality Assessment

### Tier 1: Fast Static Analysis (CURRENT)

**Purpose**: Rapid feedback for skill development iteration

**What it measures**:

- Navigation: Document structure (decision matrix, headers, links)
- Determinism: Keyword density (MUST, STOP, CRITICAL, etc.)
- Safety: Negative constraint keywords (Do not, Never, ❌)
- Cohesion: File size and structure

**Characteristics**:

- Speed: ~10ms per file
- Cost: Free (no API calls)
- Scientific Validity: **Weak** (syntax only, not agent behavior)

**When to use**:

- During skill development (fast iteration)
- CI/CD pipelines (quick smoke tests)
- Pre-commit hooks (instant feedback)

### Tier 2: Deep Verify (PLANNED - See Specification)

**Purpose**: Scientific validation of actual agent instruction following

**What it measures**:

- Real agent behavior against skill instructions
- Semantic adherence (not just keywords)
- Constraint satisfaction across test cases
- Chain-of-Thought reasoning verification

**Characteristics**:

- Speed: ~2-5s per file (100-200x slower)
- Cost: ~$0.61 per skill (OpenAI API)
- Scientific Validity: **Strong** (AdvancedIF methodology)

**When to use**:

- Production skill validation
- Critical skill auditing
- Skill quality comparisons
- Scientific research validation

**Documentation**: See `docs/skillscore-tier2-specification.md`

### Comparison Table

| Feature                 | Tier 1 (Fast)                  | Tier 2 (Deep)                    |
| ----------------------- | ------------------------------ | -------------------------------- |
| **Implementation**      | Static regex checks            | LLM-as-a-Judge (Rubric Verifier) |
| **Validates**           | Document structure             | Agent behavior                   |
| **Methodology**         | Keyword counting               | AdvancedIF rubric verification   |
| **Research Basis**      | Inspired by AGENTIF/AdvancedIF | Implements AdvancedIF protocol   |
| **Speed**               | ~10ms                          | ~2-5s                            |
| **Cost**                | Free                           | ~$0.61/skill                     |
| **Scientific Validity** | Weak (cargo culting)           | Strong (research-backed)         |
| **Use Case**            | Development iteration          | Production validation            |

**Important Note**: Tier 1 is a valuable development tool, but it does **not** measure agent instruction following quality. For scientific validation, Tier 2 (Deep Verify) is required.

## Script Architecture

The validation suite uses a **shared library** architecture to eliminate code duplication:

```
lib/skill_validator/
  ├── common.py      # Shared utilities (file finding, code stripping)
  ├── parser.py      # SKILL.md parsing logic
  ├── checks.py      # Reusable validation checks
  └── reporter.py    # Unified output formatting (console/JSON/GitLab)

lib/validate.py      # Unified CLI (replaces 4 legacy scripts)
```

## Unified CLI

The `lib/validate.py` script consolidates all validation checks:

| Check               | Purpose                     | Legacy Script                |
| ------------------- | --------------------------- | ---------------------------- |
| `broken_references` | Detect dead links           | `check_all_references.py`    |
| `empty_files`       | Find empty reference files  | `check_empty_files.py`       |
| `navigation_format` | Check for deprecated syntax | `check_navigation_format.py` |
| `depth_violations`  | Detect illegal nesting      | `check_depth_violations.py`  |

**Exit codes**: `0` (all pass) or `1` (any failure)

## Specialized Scripts

These scripts remain for specific use cases:

| Script                       | Purpose                       | When to Use                 |
| ---------------------------- | ----------------------------- | --------------------------- |
| `audit_degraded_skill.py`    | Detect excessive atomization  | Reviewing skill quality     |
| `skillscore_evaluator.py`    | Calculate SkillScore (Tier 1) | Benchmarking skills         |
| `validate_skill.py`          | Full validation workflow      | Comprehensive review        |
| `check_depth_violations.py`  | Deep topology analysis        | Debugging navigation issues |
| `check_broken_references.py` | Link integrity only           | Quick smoke test            |

## Legacy Scripts

Moved to `legacy/` folder (superseded by `lib/validate.py`):

- `check_all_references.py` → Use `lib/validate.py`
- `check_empty_files.py` → Use `lib/validate.py`
- `check_navigation_format.py` → Use `lib/validate.py`
- `quick_validate.py` → Use `lib/validate.py`

## Development

### Adding a New Check

1. Add check function to `lib/skill_validator/checks.py`:

```python
def check_your_rule(skill_root: Path) -> ValidationResult:
    result = ValidationResult("your_rule")
    # ... implement check ...
    return result
```

2. Import and add to `lib/validate.py` checks list.

### Testing

```bash
# Test on skill-creator itself
python3 lib/validate.py .

# Test on another skill
python3 lib/validate.py .claude/skills/prompt-engineering
```
