# The science behind the app

One article per question a reader asks while looking at the app: why
something looks the way it does, and how the app computes it. Each article
explains the science first, with its equations and sources, then shows how
the app turns it into a calculation and where it simplifies.

The study documents in [docs/analysis/](../analysis/) are where the app's
rules were decided. These articles explain one topic each to a reader who
wants to understand it, and link back to those studies.

## Topics

### Star view

No topic yet. Planned: "How do asteroid belts form?", "What does a G-type
star mean?"

### Planet view

- [Why are ring moons so small?](ring-moons.md)

## Writing an article

### File and title

- One file per topic: `docs/science/<topic>.md`, in kebab case
  (`ring-moons.md`). Images go in `docs/science/images/`, named after the
  topic (`ring-moons-saturn.svg`).
- The title is the reader's question, as they would ask it ("Why are ring
  moons so small?").
- Add the article to the topics above, under the view where the question
  comes up.
- Write in English, like the rest of the repository's documentation.

### Outline

Every article has these sections, in this order:

1. **Title and "Seen from"**: the question, then one line saying where in
   the app the reader meets it (the view, the card, the panel).
2. **Short answer**: three to five sentences for the reader who only wants
   the gist. No equations.
3. **The science**: what the theories say, in subsections. Each equation is
   followed by:
   - **In words**: what it says, without symbols.
   - **Orders of magnitude**: real numbers from a known body (Saturn, the
     Moon, the Sun) that the reader can picture.

   End with **What we do not know** when the topic is debated or uncertain.
4. **In the app**:
   - **What you see**: where the topic shows in the app and what it looks
     like.
   - **How it computes them**: the formula the app keeps, its steps, an
     excerpt of the code (trimmed, with comments) and links to the source
     files.
   - **Where it departs from the science**: each simplification, what it
     changes, and when it would show. Say it plainly: this section is what
     makes the article honest.
5. **Sources**: a numbered list.
6. **See also**: the study in `docs/analysis/`, related articles, the spec.

### Equations

- Display equations go in fenced `math` blocks, which GitHub renders:

  ````markdown
  ```math
  r_H = a\left(\frac{m}{3M}\right)^{1/3}
  ```
  ````

- Inline symbols go between dollar signs: `$\rho_p$`.
- Define every symbol in the sentence before the equation.

### Diagrams

Add a diagram whenever a picture explains faster than a paragraph.

- **Mermaid** (a fenced `mermaid` block) for a process, a sequence of steps
  or a cause and its effects.
- **SVG** in `docs/science/images/` for geometry: distances, orbits, sizes.
  - Draw it to scale when you can, and say what is and is not to scale.
  - Give it an opaque background and light text, so it reads in GitHub's
    light and dark themes alike.
  - Give it a `<title>` and a `<desc>` that say what it shows, and alt text
    in the Markdown image.
  - Put the sources of its numbers in a footer line inside the image.
- Check the rendering before committing (open the SVG in a browser).

### Sources

- Cite a source for every number and every claim in **The science**, in
  the text as "Author et al. year", with the full reference in **Sources**.
- Prefer peer-reviewed papers and textbooks. Link each paper by its DOI,
  and add its arXiv preprint when it has one. Agency data (NASA, ESA) is
  fine for measured values.
- Wikipedia may lead you to the sources but is never cited as one.
- Quote a paper only with its exact words, in quotation marks.
- Check each number against the paper itself, not a press release. When
  you could not, say so or leave the number out.

### Code excerpts

- Trim the excerpt to the lines that carry the idea, keep the source's
  names, and say which file it comes from.
- Link source files with relative paths
  (`../../crates/mapgen/src/rings.rs`), not line numbers, which drift.
- When the code changes the behaviour an article describes, update the
  article in the same change.
