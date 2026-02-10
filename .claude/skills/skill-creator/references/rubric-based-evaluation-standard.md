# Rubric-Based Evaluation: The New Standard for Agent Skills

Based on 2025/2026 research (AGENTIF, AdvancedIF), we implemented a rubric-based `SkillScore`.

## The "AgentIF" Standard

Research shows that agent reliability depends on **Instruction Success Rate (ISR)**, not just similarity. To achieve high ISR, instructions must be:

1. **Decomposable**: Broken into atomic constraints.
2. **Deterministic**: Uses imperative language (`MUST`, `STOP`).
3. **Navigable**: Reduces search space via routing.
4. **Bounded**: Explicit negative constraints (`DO NOT`).

## The SkillScore Rubric

We implement a deterministic rubric evaluator (`skillscore_evaluator.py`) that measures these exact qualities.

| Category        | Weight  | Criteria                                      | Research Basis                                        |
| :-------------- | :------ | :-------------------------------------------- | :---------------------------------------------------- |
| **Navigation**  | **40%** | `<decision_matrix>`, Headers, Tags            | Reduces "Lost in the Middle" phenomenon (AGENTIF).    |
| **Determinism** | **30%** | Imperative verbs (`MUST`, `STOP`, `CRITICAL`) | Increases constraint adherence by ~40% (AdvancedIF).  |
| **Safety**      | **15%** | Negative constraints (`DO NOT`, `NEVER`)      | Explicit boundaries required for safety (AdvancedIF). |
| **Cohesion**    | **15%** | Semantic density (50-2000 lines)              | Optimizes context window usage.                       |

## Implementation Plan

1. **Metric**: `SkillScore` (0-100).
2. **Tool**: `scripts/skillscore_evaluator.py` (Python script).
3. **Integration**: Integrated `skillscore_evaluator.py` in `validate_skill.py`.
4. **Docs**: Updates `validation-quality-gates.md`.

This moved us from "Vibe Checks" to "Engineering Standards" (Rubrics).
