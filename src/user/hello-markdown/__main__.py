# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

import markdown

def main():
    print("Converting Markdown to HTML...")

    # Sample markdown content
    md_text = """
# Hello from Nanvix!

This is a **bold** statement and this is *italic*.

## Features

- Pure Python markdown library
- No C extensions required
- Works in Nanvix VM

## Code Example

```python
print("Hello, World!")
```

> This is a blockquote from inside the Nanvix microkernel.

Visit [Nanvix](https://github.com/nanvix/nanvix) for more info.
"""

    # Convert to HTML
    html = markdown.markdown(md_text, extensions=['fenced_code'])

    print("Generated HTML:")
    print(html)
    print()
    print("ok")

if __name__ == "__main__":
    main()
