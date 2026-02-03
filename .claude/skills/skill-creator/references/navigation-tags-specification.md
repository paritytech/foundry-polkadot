<atom>
  <type>reference</type>
  <diataxis_quadrant>reference</diataxis_quadrant>
  <intent>navigation_tag_syntax</intent>
</atom>

# Navigation Tags Syntax

## Tag Format

Navigation tags use angle brackets with action targets:

```markdown
<navigate_to>references/specific-file.md</navigate_to>
<delegate_to>agent-name</delegate_to>
```

## Standard Tags

| Tag               | Purpose                        | Example                                   |
| :---------------- | ------------------------------ | ----------------------------------------- |
| navigate_to       | Link to another reference file | See decision matrix below                 |
| delegate_to       | Delegate to an external agent  | librarian, oracle, explore                |
| external_doc      | Query external documentation   | Context7, official docs, package managers |
| source_inspection | Inspect source code directly   | Effect node_modules, Rust crates, etc.    |

## Source Inspection Tag

For direct source code inspection in package repositories (node_modules, vendor, etc.):

```markdown
<option condition="Understand Effect-TS internal implementation details">
  <source_inspection script=".claude/skills/effect-ts/scripts/find_effect_packages.py" args="[directory]" />
  <description>Deep inspection of Effect-TS source code for type signatures, internal algorithms, and implementation details.</description>
</option>
```

**Attributes**:

- `script`: Path to discovery script (required)
  - Must be relative to project root
  - Script must be executable (shebang present)
- `args`: Command-line arguments (optional)
  - Defaults depend on script (e.g., `./node_modules` for find_effect_packages.py)

**Usage Guidelines**:

- Use `<source_inspection>` for understanding implementation details, type signatures, internal algorithms
- Complement `<external_doc>` (official docs) with `<source_inspection>` (source code)
- Scripts should output structured data (tab-separated, JSON) for parsing
- Use as a first-class discovery method, not a fallback

**When to use**:

- Official documentation is unclear or incomplete
- Need to understand type-level details or implementation
- Debugging library behavior from source
- Verifying API contracts or edge cases

## External Documentation Tag

For querying external documentation systems (Context7, official docs, package managers):

```markdown
<option condition="Define a service with typed methods and dependencies">
  <external_doc system="context7" libraryId="/effect-ts/effect" query="Context.Tag Effect.Service" />
  <navigate_to>references/service-architecture-comprehensive.md</navigate_to>
  <description>Context7 for API syntax. Local file for project-specific patterns.</description>
</option>
```

**Attributes**:

- `system`: Documentation system name (required)
  - Valid values: `context7`, `github`, `official-docs`, `pkg-manager`
- `libraryId`: Unique identifier for the library/package (required)
  - For `context7`: Must start with `/` (e.g., `/effect-ts/effect`)
- `query`: Search query or topic (required)
  - Minimum 5 characters

**Usage Guidelines**:

- Use `<external_doc>` when official documentation exists for the topic
- Combine with `<navigate_to>` for project-specific patterns
- Options without external docs should only use `<navigate_to>`
- Self-closing tag (no closing `</external_doc>` needed)

## Decision Matrix Navigation

Most common pattern - option blocks with action targets:

```markdown
<option condition="User intent description">
  <action>
    <navigate_to>references/workflow-file.md</navigate_to>
    <navigate_to>references/another-file.md</navigate_to>
  </action>
</option>
```

## Delegation

External agent delegation for specialized tasks:

```markdown
<option condition="Complex architectural decision">
  <action><delegate_to>oracle</delegate_to></action>
</option>
```

## Validation Rules

1. File paths must be relative to skill root
2. Use `references/` prefix for reference files
3. Agent names must match valid agents (explore, librarian, oracle, etc.)
4. Tags cannot be nested
5. All navigation must be inside `<action>` blocks
