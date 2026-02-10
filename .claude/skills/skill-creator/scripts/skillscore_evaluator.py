#!/usr/bin/env python3
"""
SkillScore Evaluator: Rubric-Based Benchmarking for Agent Skills

DISCLAIMER: This is a TIER 1 (Static Analysis) tool.
It checks for structural anchors and keyword density to provide fast, local feedback.
It does NOT perform TIER 2 (Semantic Verification) using LLM-as-a-Judge.
High scores indicate good *structure*, not necessarily good *reasoning*.

Based on "AdvancedIF" and "AGENTIF" research:
- Navigation (40%): Does the agent know where to go?
- Determinism (30%): Are instructions imperative and clear?
- Safety (15%): Are negative constraints explicit?
- Cohesion (15%): Is the content structure sound?
"""

import sys
import re
import argparse
from pathlib import Path
from typing import Dict, Any, List

# Rubric Weights
WEIGHTS = {
    "navigation": 0.40,
    "determinism": 0.30,
    "safety": 0.15,
    "cohesion": 0.15,
}


def evaluate_navigation(content: str) -> Dict[str, Any]:
    """
    Checks for structural anchors that guide agent latent space.
    - Decision Matrix (Critical)
    - Navigation Tags (JIT Loading)
    - Clear Headers
    """
    score = 0
    issues = []

    # Check 1: Decision Matrix (The primary router) - 50 pts
    if "<decision_matrix>" in content and "</decision_matrix>" in content:
        score += 50
    else:
        issues.append("Missing <decision_matrix>. Agents need explicit routing logic.")

    # Check 2: Navigation Tags or Strong Headers - 30 pts
    # We accept either XML tags OR clear markdown structure with TOC
    has_nav_tags = "<nav>" in content
    has_toc = (
        "Table of Contents" in content
        or "## Index" in content
        or "## Contents" in content
    )
    has_headers = len(re.findall(r"^##\s+", content, re.MULTILINE)) > 2

    if has_nav_tags:
        score += 30
    elif has_toc and has_headers:
        score += 30
    elif has_headers:
        score += 15
        issues.append("Weak navigation. Consider adding <nav> tags or a TOC.")
    else:
        issues.append("Poor document structure. Few headers found.")

    # Check 3: Valid Links - 20 pts
    links = re.findall(r"\[.*?\]\((.*?)\)", content)
    if len(links) > 0:
        score += 20
    else:
        # It's okay if a file has no links if it's a leaf node, but usually skills have links
        score += 10  # Half credit benefit of doubt

    return {"score": min(100, score), "issues": issues}


def evaluate_determinism(content: str) -> Dict[str, Any]:
    """
    Checks for imperative ("Command") language.
    Agents follow orders, not suggestions.
    """
    score = 0
    issues = []

    # Check 1: Imperative Verb Density - 60 pts
    # We look for "MUST", "STOP", "CRITICAL", "ALWAYS", "NEVER"
    imperatives = [
        "MUST",
        "STOP",
        "CRITICAL",
        "ALWAYS",
        "NEVER",
        "REQUIRED",
        "MANDATORY",
    ]
    found_imperatives = [w for w in imperatives if w in content]

    if len(found_imperatives) >= 3:
        score += 60
    elif len(found_imperatives) > 0:
        score += 30
        issues.append(
            f"Weak command density. Found only: {found_imperatives}. Use more: MUST, STOP, CRITICAL."
        )
    else:
        issues.append(
            "No imperative keywords found. Use strong constraints (MUST, CRITICAL)."
        )

    # Check 2: Structured Output Definition - 40 pts
    # Look for code blocks showing examples, schemas, or templates
    has_code_block = "```" in content
    has_template = "Template" in content or "Structure" in content

    if has_code_block or has_template:
        score += 40
    else:
        issues.append("No output templates or code blocks found. Agents need schemas.")

    return {"score": min(100, score), "issues": issues}


def evaluate_safety(content: str) -> Dict[str, Any]:
    """
    Checks for negative constraints and boundaries.
    "What NOT to do" is as important as "What to do".
    """
    score = 0
    issues = []

    # Check 1: Negative Constraints - 60 pts
    # "Do not", "Avoid", "Forbidden", "Never"
    negatives = ["Do not", "do not", "Avoid", "Forbidden", "Never", "NEVER", "❌"]
    count = sum(1 for w in negatives if w in content)

    if count >= 2:
        score += 60
    else:
        issues.append(
            "Few negative constraints. Explicitly state what the agent MUST NOT do."
        )

    # Check 2: Warning Labels - 40 pts
    # "Warning", "Caution", "Alert", "Risk"
    warnings = ["Warning", "Caution", "Alert", "Risk", "Note:", "Important:"]
    if any(w in content for w in warnings):
        score += 40
    else:
        score += 40  # Giving pass here as not all docs need warnings, but good to check
        # No issue appended to avoid noise

    return {"score": min(100, score), "issues": issues}


def evaluate_cohesion(content: str) -> Dict[str, Any]:
    """
    Checks file size and semantic density.
    """
    score = 100
    issues = []
    line_count = len(content.splitlines())

    # Penalty for micro-files (< 100 lines) unless it looks like a simple reference
    if line_count < 50:
        score = 50
        issues.append(f"File too small ({line_count} lines). Risk of fragmentation.")

    # Penalty for monoliths (> 2000 lines)
    if line_count > 2000:
        score = 60
        issues.append(f"File too large ({line_count} lines). Risk of context flooding.")

    return {"score": score, "issues": issues}


def calculate_skillscore(file_path: str) -> Dict[str, Any]:
    path = Path(file_path)
    if not path.exists():
        return {"error": f"File not found: {file_path}"}

    content = path.read_text(encoding="utf-8")

    nav = evaluate_navigation(content)
    det = evaluate_determinism(content)
    safe = evaluate_safety(content)
    coh = evaluate_cohesion(content)

    final_score = (
        nav["score"] * WEIGHTS["navigation"]
        + det["score"] * WEIGHTS["determinism"]
        + safe["score"] * WEIGHTS["safety"]
        + coh["score"] * WEIGHTS["cohesion"]
    )

    return {
        "file": path.name,
        "skill_score": round(final_score, 1),
        "breakdown": {
            "navigation": nav,
            "determinism": det,
            "safety": safe,
            "cohesion": coh,
        },
    }


def print_report(result: Dict[str, Any]):
    if "error" in result:
        print(f"[ERROR] {result['error']}")
        return

    score = result["skill_score"]
    color = "\033[92m" if score >= 80 else "\033[93m" if score >= 60 else "\033[91m"
    reset = "\033[0m"

    print(f"\nSkillScore Report for: {result['file']}")
    print(f"Final Score: {color}{score}/100{reset}")
    print("-" * 40)

    for category, data in result["breakdown"].items():
        cat_score = data["score"]
        weight = int(WEIGHTS[category] * 100)
        print(f"{category.title():<12} ({weight}%): {cat_score:>3}/100")
        for issue in data["issues"]:
            print(f"  - {issue}")
    print("-" * 40)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Calculate SkillScore for a markdown file"
    )
    parser.add_argument("file_path", help="Path to the markdown file (SKILL.md)")
    args = parser.parse_args()

    report = calculate_skillscore(args.file_path)
    print_report(report)
