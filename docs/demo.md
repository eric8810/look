# dlook

Fast terminal previewer: markdown, code & mermaid.

## Rendering

- [x] colored headings (cyan / blue)
- [x] task lists, links, rounded tables
- [ ] your file, beautifully rendered

## Code — syntect truecolor

```rust
fn main() {
    let doc = dlook::open("demo.md")?;
    doc.render(); // 24-bit colors
}
```

| content | engine | colors |
| --- | --- | --- |
| markdown | termimad | cyan/blue |
| code | syntect | truecolor |
| mermaid | mermansi | truecolor |

Docs: [README](https://github.com/eric8810/look) — drag to select, release to copy.
