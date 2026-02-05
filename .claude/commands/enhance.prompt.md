---
description: Transform arbitrary user prompts into high-fidelity enhanced prompts following the Diátaxis + C.L.E.A.R. framework from the prompt-engineering skill
---

# Prompt Enhancement Command

<mandate>
**MANDATORY: SKILL INVOCATION**

Before any processing, you MUST invoke the prompt-engineering skill:

```
skill(name="prompt-engineering")
```

**Do not proceed without the skill loaded.**
</mandate>

---

## Context

This command operates as a meta-prompt enhancer. Users provide raw prompts that require transformation into expert-level instructions using the prompt-engineering framework.

## Objective

Transform user-provided prompts into structured, high-fidelity prompts and save them to files.

---

## Decision Matrix: Enhancement Strategy

<decision_matrix>
<question>What type of prompt enhancement does the user's input require?</question>

<option condition="User input is completely vague (single word, &quot;help me&quot;, &quot;fix this&quot; with no context)">
  <navigate_to>#ambiguity-protocol</navigate_to>
  <description>Input is undecipherable - request clarification before attempting enhancement</description>
</option>

<option condition="User input has clear intent but missing critical details (what, who, format undefined)">
  <navigate_to>#phase-2-diataxis-selection</navigate_to>
  <description>Proceed with enhancement - infer missing context or mark with placeholders</description>
</option>

<option condition="User input is well-specified with clear intent, audience, and format requirements">
  <navigate_to>#phase-3-clear-optimization</navigate_to>
  <description>Apply full framework optimization - skip ambiguity handling</description>
</option>

<option condition="User input requires domain-specific knowledge (code, medical, legal, technical)">
  <navigate_to>#phase-5-advanced-techniques</navigate_to>
  <description>Add domain-specific Few-Shot examples and Chain-of-Thought reasoning</description>
</option>
</decision_matrix>

---

## Workflow Execution

### Phase 0: Skill Verification

**Before proceeding, confirm:**

- [ ] `skill(name="prompt-engineering")` has been executed
- [ ] Skill framework documentation is available
- [ ] Decision matrix has routed you to the appropriate phase

**If skill not loaded**: Return to `<mandate>` section and invoke immediately.

---

### Ambiguity Protocol

<a id="ambiguity-protocol"></a>

If the user's intent is completely undecipherable, do NOT generate an enhanced prompt. Instead:

```markdown
## Clarification Required

The input is too vague to enhance effectively. Please provide more context:

**Question 1**: What specific task do you need help with?

- Example: "Write code for X", "Analyze data from Y"

**Question 2**: Who is the target audience for this task?

- Example: "Beginner developers", "Data scientists"

**Question 3**: What output format do you prefer?

- Example: "Markdown", "JSON", "Python code"

Please re-run the command with these details.
```

**Stop execution after requesting clarification.**

---

### Phase 1: Intent Analysis

Analyze the user's raw prompt to identify:

1. **Core Intent**: What is the user actually asking for?
2. **Task Type**: Content generation? Code execution? Analysis? Creative work?
3. **Complexity Level**: Simple, moderate, or complex reasoning required?
4. **Missing Information**: What context or constraints are undefined?

**Document these findings in your analysis output.**

---

### Phase 2: Diátaxis Mode Selection

<a id="phase-2-diataxis-selection"></a>

**Determine which Diátaxis quadrant the enhanced prompt should use:**

| Mode            | Trigger Keywords                          | Persona Archetype           | Key Characteristic                       |
| :-------------- | :---------------------------------------- | :-------------------------- | :--------------------------------------- |
| **Tutorial**    | "teach me", "tutorial", "learn"           | Mentor / Teacher            | Learning-oriented, step-by-step          |
| **How-To**      | "how do I", "how to", "fix", "implement"  | Senior Engineer / Colleague | Task-oriented, imperative, concise       |
| **Reference**   | "list", "show", "parameters", "api"       | Librarian / Database        | Information-oriented, structured, dry    |
| **Explanation** | "what is", "why", "explain", "understand" | Professor / Philosopher     | Context-oriented, discursive, generative |

**Critical**: Select ONE quadrant only. Mixed modes fail at both.

> **Skill Reference**: See `/skill prompt-engineering → references/diataxis-framework.md`

---

### Phase 3: C.L.E.A.R. Optimization

<a id="phase-3-clear-optimization"></a>

**Apply the five principles systematically:**

| Principle      | Action                                       | Measurable Impact                 |
| :------------- | :------------------------------------------- | :-------------------------------- |
| **Concise**    | Strip fillers ("please", "thank you")        | 70% token reduction               |
| **Logical**    | Order: Context → Task → Constraints → Format | Reduces context fragmentation     |
| **Explicit**   | Objective metrics (word counts, schemas)     | 91% hallucination reduction       |
| **Adaptive**   | IF/THEN logic for edge cases                 | 30% fewer edge case failures      |
| **Reflective** | Self-correction before output                | 67% reduction in reasoning errors |

> **Skill Reference**: See `/skill prompt-engineering → references/clear-framework.md`

---

### Phase 4: COSTAR Structure

**Build the enhanced prompt using COSTAR elements:**

```xml
<context>
[Background information, relevant context, environment details]
</context>

<objective>
[Clear, measurable goal with success criteria]
</objective>

<style>
[Tone: formal/technical/concise/educational/etc.]
[Voice: expert/colleague/mentor/etc.]
</style>

<tone>
[Professional level: academic/practical/casual/etc.]
[Emotion: neutral/enthusiastic/urgent/etc.]
</tone>

<audience>
[Target audience expertise level: beginner/intermediate/expert]
[Domain knowledge assumptions]
</audience>

<response_format>
[Output structure: JSON/Markdown/code blocks/etc.]
[Length constraints: word counts, token limits]
[Specific sections required]
</response_format>
```

> **Skill Reference**: See `/skill prompt-engineering → references/costar-framework.md`

---

### Phase 5: Advanced Techniques

<a id="phase-5-advanced-techniques"></a>

**Based on task type, add appropriate techniques:**

**For Complex Reasoning (Chain-of-Thought):**

```xml
<thinking>
Think step-by-step through this problem:
1. What are the key constraints?
2. What approach best fits the requirements?
3. What edge cases need consideration?
4. Verify your solution against the requirements
</thinking>
```

**For Pattern Enforcement (Few-Shot):**

```xml
<examples>
**Example 1: [Description]**
Input: [Sample input]
Output: [Expected output with reasoning]
</examples>
```

**For Security (Delimiters):**

```xml
<instructions>
[System instructions here]
</instructions>

<context>
[Background information]
</context>

<user_input>
[User's actual data]
</user_input>
```

> **Skill Reference**: See `/skill prompt-engineering → references/reasoning-techniques.md`

---

### Phase 6: Reflexion and Self-Correction

**Before finalizing, critique the enhanced prompt:**

**Diátaxis Completeness:**

- [ ] Single quadrant selected (not mixed)
- [ ] Persona matches quadrant
- [ ] Negative constraints inhibit wrong-mode content

**C.L.E.A.R. Adherence:**

- [ ] No filler words
- [ ] Logical order: Context → Task → Constraints → Format
- [ ] Objective metrics (no "brief", "clear", "appropriate")
- [ ] IF/THEN logic for edge cases
- [ ] Reflection block before final output

**COSTAR Structure:**

- [ ] All 6 elements present
- [ ] XML delimiters properly structured
- [ ] Audience and tone match Diátaxis persona

**Execution Safety:**

- [ ] Output is a REWRITTEN PROMPT, not an answer
- [ ] No actual content generated per user's request
- [ ] Clear separation between template and data

**Scope Preservation (Anti-Planner):**

- [ ] Enhanced prompt maintains original abstraction level
- [ ] No new implementation details added beyond original request
- [ ] Technical specifications only enhance what user already provided
- [ ] Boundaries clarify scope, don't dictate implementation approach

---

## CRITICAL EXECUTION RULES

<execution-warning>
**DO NOT EXECUTE THE USER'S PROMPT**

Your ONLY task is to REWRITE and ENHANCE the user's prompt. You must NEVER:

- Answer the user's prompt
- Perform the task requested in the user's prompt
- Generate code, content, or solutions requested
- Execute any actions the user's prompt describes

**EXAMPLES OF CORRECT BEHAVIOR:**
✅ User: "Write a Python function to sort a list"
→ You: Rewrite to: "Act as a Python expert. Write a function that sorts a list..."
→ You: SAVE the rewritten prompt to a file
→ You: DO NOT actually write the Python function

**EXAMPLES OF INCORRECT BEHAVIOR:**
❌ User: "Write a Python function"
→ You: Write actual Python code (WRONG - this is executing)
</execution-warning>

---

## AVOID: The Planner Anti-Pattern

**CRITICAL: Add CLARITY, Not Implementation Details**

Your task is to enhance prompt clarity and structure, NOT to add fine-grained implementation planning. The enhanced prompt should remain at the same level of abstraction as the original input.

**What This Means:**

✅ **GOOD: Preserve Original Scope**

- Original: "Implement a user authentication system"
- Enhanced: "Implement a user authentication system. Focus on security, usability, and standard authentication patterns. Exclude edge cases around third-party OAuth providers."

❌ **BAD: Over-Specification (The Planner Trap)**

- Original: "Implement a user authentication system"
- Enhanced (WRONG): "Implement a user authentication system with JWT tokens stored in Redis, using bcrypt with 12 rounds, implementing refresh token rotation every 24 hours..."

**Reflexion Check:**

1. Did I add technical specifications that weren't in the original prompt?
2. Am I dictating HOW to implement, rather than WHAT to implement?
3. Is the enhanced prompt more prescriptive than the user's request?

If yes to any, remove the added specificity and focus on clarity instead.

---

## Enhanced Prompt Structure

Save the enhanced prompt with this structure:

````markdown
# Diátaxis + C.L.E.A.R. + COSTAR Enhancement Analysis

## Original Prompt

[The user's raw input exactly as provided]

## Analysis

### Intent

[Core intent identified in 1-2 sentences]

### Diátaxis Mode

[Selected quadrant and why]

### Task Type

[Content generation / Code execution / Analysis / Creative / Other]

### Missing Information

[List of gaps inferred or marked with `{{INSERT_...}}` placeholders]

## Enhanced Prompt

```xml
<context>
[Background context from COSTAR]
</context>

<objective>
[Clear objective from COSTAR]
</objective>

<style>
[Tone and voice specifications]
</style>

<tone>
[Professional and emotional tone]
</tone>

<audience>
[Target audience expertise level]
</audience>

<response_format>
[Specific output structure requirements]
</response_format>

[Additional Diátaxis + C.L.E.A.R. elements as needed]
```
````

## Framework Improvements

### Diátaxis Components Applied

#### Mode Selection

[Which quadrant and why]

#### Persona Assignment

[Expert archetype assigned]

### C.L.E.A.R. Components Applied

#### C - Concise

[How token efficiency was improved]

#### L - Logical

[How sequencing was optimized]

#### E - Explicit

[How output space was defined]

#### A - Adaptive

[What edge case logic was added]

#### R - Reflective

[What quality gates were added]

### COSTAR Components Applied

#### Context

[What background information was structured]

#### Objective

[How the goal was clarified]

#### Style

[What tone and voice were specified]

#### Tone

[What professional level was set]

#### Audience

[How expertise assumptions were defined]

#### Response Format

[How output structure was constrained]

### Advanced Techniques

#### Chain of Thought

[CoT instructions added (if applicable)]

#### Few-Shot Examples

[Examples generated and what they demonstrate]

#### Delimiters

[How delimiters structure the prompt]

### Ambiguity Resolution

[How gaps were filled or marked for user input]

## Usage Notes

### Iteration Suggestions

[How to refine the prompt based on initial results]

### Customization Points

[`{{PLACEHOLDERS}}` to fill before use]

### Common Adjustments

[Typical modifications for different use cases]

````
---

## File Naming Convention

Generate safe filenames automatically:

**Algorithm:**
1. Extract 3-5 meaningful content words from the prompt
2. Convert to lowercase
3. Replace spaces with hyphens
4. Remove special characters (except hyphens and underscores)
5. Append `-enhanced.txt` suffix

**Examples:**
- "How do I implement OAuth2 in Node.js" → `oauth2-implementation-nodejs-enhanced.txt`
- "Create a marketing plan for coffee shop" → `coffee-shop-marketing-plan-enhanced.txt`
- "Analyze this Python code for bugs" → `python-code-bug-analysis-enhanced.txt`

**Save Location:**
- Current working directory
- Overwrite existing file with same name without confirmation

---

## Response Format

### Primary Output Structure

**To User (Terminal):**

1. **Analysis section**: Core intent, missing information
2. **Diátaxis quadrant identification**: Which mode and why
3. **C.L.E.A.R. breakdown**: What was enhanced and why
4. **COSTAR structure**: How elements were applied
5. **Framework improvements**: Detailed breakdown of enhancements
6. **Usage notes**: Iteration and customization guidance

**To File (Saved to Disk):**

- ONLY the clean, enhanced prompt itself
- No analysis, no explanations, no original prompt
- Copy-paste ready for immediate LLM use

### File Output Details

- Filename: `{topic}-enhanced.txt` (auto-generated)
- Location: Current working directory
- Overwrite: Existing files with same name without confirmation
- Content: ONLY the enhanced prompt with XML/Markdown delimiters
- NO meta-discussion, NO headers like "## Enhanced Prompt"

---

## Completion Report

After saving the file, provide this structured report:

```markdown
## Enhancement Complete

**File Saved:** [Full file path]

### Diátaxis Summary

- **Mode**: [Selected quadrant]
- **Persona**: [Expert archetype]
- **Rationale**: [Why this mode was chosen]

### C.L.E.A.R. Summary

- ✅ **Concise**: [Token reduction achieved]
- ✅ **Logical**: [Flow optimization applied]
- ✅ **Explicit**: [Output space defined]
- ✅ **Adaptive**: [Edge case handling added]
- ✅ **Reflective**: [Quality gates implemented]

### COSTAR Summary

- ✅ **Context**: [Background structured]
- ✅ **Objective**: [Goal clarified]
- ✅ **Style**: [Tone/voice specified]
- ✅ **Tone**: [Professional level set]
- ✅ **Audience**: [Expertise defined]
- ✅ **Response Format**: [Output constrained]

### Techniques Applied

- [Diátaxis quadrant selection]
- [C.L.E.A.R. principles]
- [COSTAR structure]
- [Chain of Thought (if applicable)]
- [Few-Shot Prompting (if applicable)]
- [XML Delimiters]

### Usage Recommendation

[1-2 sentences on how to use the enhanced prompt effectively]

### Note

This enhanced prompt is ready for use with high-performance LLMs. The saved file contains ONLY the clean enhanced prompt - no analysis, no explanations, no meta-discussion. Copy and paste the file contents directly into your LLM prompt input.
```

---

## Skill Reference

This command implements the workflow for the **`/skill prompt-engineering`** methodology. The skill provides comprehensive framework documentation:

- **Diátaxis**: `references/diataxis-framework.md` - Four quadrants for information architecture
- **C.L.E.A.R.**: `references/clear-framework.md` - Five principles for prompt optimization
- **COSTAR**: `references/costar-framework.md` - Six-element structure for comprehensive prompts
- **Chain-of-Thought**: `references/chain-of-thought.md` - Logical decomposition and Tree of Thoughts
- **Reasoning**: `references/reasoning-techniques.md` - Decision Matrices and Few-Shot patterns

**This command provides the workflow and execution rules** for applying those frameworks.
````
