# typst_po

Cross-platform GNU gettext translation tooling for [Typst](https://typst.app/).

* **`typ2pot`**: Scans Typst files and extracts paragraphs, headings, and lists into a `.pot` template while ignoring code logic (`#let`, `#show`, `#set`).
* **`typ-translate`**: Takes your source `.typ` tree and a translated `.po` file, translates strings using lossless byte-offset splicing, and outputs a mirrored folder in `build/<lang>/`.

---

## 1. Installation & Building

Prerequisites: [Install Rust](https://www.rust-lang.org/tools/install) (Cargo).

```bash
git clone https://github.com/ExeVirus/typst_po.git
cd typst_po
cargo build --release
```

The compiled binaries will be in `./target/release/`:
* **Linux / macOS**: `./target/release/typ2pot` and `./target/release/typ-translate`
* **Windows**: `.\target\release\typ2pot.exe` and `.\target\release\typ-translate.exe`

*(Optional) Copy or symlink them to your system PATH.*

---

## 2. Quickstart Workflow

### Step 1: Extract `.pot` template from your Typst source
Scan a single file or an entire multi-file documentation folder:

**Linux / macOS:**
```bash
./target/release/typ2pot src/ -o locales/messages.pot
```

**Windows (PowerShell):**
```powershell
.\target\release\typ2pot.exe src\ -o locales\messages.pot
```

### Step 2: Initialize or update your `.po` file
Use standard GNU gettext tools or GUI editors like [Poedit](https://poedit.net/):

```bash
# Initialize a new German translation (or copy messages.pot to de.po)
msginit -i locales/messages.pot -o locales/de.po -l de

# Or update an existing translation with new template strings:
msgmerge -U locales/de.po locales/messages.pot
```

Fill in the `msgstr` translations inside `locales/de.po`.

### Step 3: Build the translated Typst project
This reads your source folder and generates a mirrored build folder with all translated `.typ` files (assets and relative `#include` / `#import` paths are preserved automatically):

**Linux / macOS:**
```bash
./target/release/typ-translate src/ locales/de.po -o build/de/
```

**Windows (PowerShell):**
```powershell
.\target\release\typ-translate.exe src\ locales\de.po -o build\de\
```

### Step 4: Compile to PDF with Typst
Compile the generated document using the official Typst compiler:

**Linux / macOS:**
```bash
typst compile --root . build/de/main.typ dist/document-de.pdf
```

**Windows (PowerShell):**
```powershell
typst compile --root . build\de\main.typ dist\document-de.pdf
```