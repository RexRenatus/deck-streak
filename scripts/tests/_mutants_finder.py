"""The reader of a workflow's `run:` text and the finder of its `cargo mutants` commands (SPEC-129).

One home for the finder both workflow guards use: `test_dispatch_shards.py` and
`test_mutation_workflows.py` import it, and neither keeps a copy. It is moved out of
`test_dispatch_shards.py`, not rewritten. A command is also found where a wrapper runs it, and
what a wrapper is belongs to the caller: `wrapped` is a parameter of the finder, never a module
attribute, so what one importer passes cannot reach another in the same process.
"""

import re
import string

BOUNDS = "--timeout 2300 --build-timeout 600"


def no_wrapper(words, program):
    """The recognizer that sees no wrapper: where the command begins is the program word itself."""
    return None


# The reader (#395, #447). A `run:` value reaches bash in three steps, and the reader takes each as
# its grammar defines it, or it refuses: YAML 1.2.2 decodes the scalar, GitHub substitutes each
# `${{ }}` expression, and bash splits the text into commands and words (POSIX XCU 2.3 and bash's
# quoting). A refusal is reported as a command that carries no bounds, so it fails the guard too.


class Refused(Exception):
    """A workflow the reader cannot read exactly; the message says where and why."""


YAML_KEY = re.compile(r"([A-Za-z0-9_][A-Za-z0-9_.-]*):(?:[ \t]+|$)")
BLOCK_HEADER = re.compile(r"([|>])(?:([1-9])([-+]?)|([-+])([1-9]?))?[ \t]*(?:#.*)?$")
FLOW_LIST = re.compile(r"\[([^\[\]{}'\"#]*)\][ \t]*(?:#.*)?$")
YAML_ESCAPES = dict(zip('0abt\tnvfre "/\\N_LP', '\0\a\b\t\t\n\v\f\r\x1b "/\\\x85\xa0\u2028\u2029'))
YAML_HEX = {"x": 2, "u": 4, "U": 8}


def indent_of(line):
    return len(line) - len(line.lstrip(" "))


def flow_scalar(lines, k, i, parent):
    """(value, line, column after it) of the quoted scalar opening at `lines[k][i]`, folded as
    YAML folds a flow scalar: a line break is a space, each empty line a newline."""
    quote, line, i, out, blanks = lines[k][i], lines[k], i + 1, [], ""
    while True:
        if i >= len(line):
            k, empty = k + 1, 0
            while k < len(lines) and not lines[k].strip(" \t"):
                k, empty = k + 1, empty + 1
            if k == len(lines) or indent_of(lines[k]) <= parent or lines[k].startswith("---"):
                raise Refused(f"line {k}: a quoted scalar this reader cannot close")
            out.append("\n" * empty or " ")
            line, i, blanks = lines[k], len(lines[k]) - len(lines[k].lstrip(" \t")), ""
            continue
        c = line[i]
        if c in " \t":
            blanks, i = blanks + c, i + 1
        elif c == quote and not (quote == "'" and line.startswith("''", i)):
            return "".join(out) + blanks, k, i + 1
        elif quote == "'":
            out.append(blanks + c)
            blanks, i = "", i + (2 if c == "'" else 1)
        elif c != "\\":
            out.append(blanks + c)
            blanks, i = "", i + 1
        elif i + 1 == len(line):
            if k + 1 == len(lines) or not lines[k + 1].strip(" \t"):
                raise Refused(f"line {k}: an escaped line break before an empty line")
            out.append(blanks)
            k, blanks = k + 1, ""
            line, i = lines[k], len(lines[k]) - len(lines[k].lstrip(" \t"))
        elif line[i + 1] in YAML_ESCAPES:
            out.append(blanks + YAML_ESCAPES[line[i + 1]])
            blanks, i = "", i + 2
        elif line[i + 1] in YAML_HEX:
            n = YAML_HEX[line[i + 1]]
            digits = line[i + 2 : i + 2 + n]
            if len(digits) < n or not all(d in "0123456789abcdefABCDEF" for d in digits):
                raise Refused(f"line {k}: a malformed escape")
            out.append(blanks + chr(int(digits, 16)))
            blanks, i = "", i + 2 + n
        else:
            raise Refused(f"line {k}: an escape YAML does not define")


def block_scalar(lines, k, header, parent):
    """(value, next line) of the literal or folded block whose header `header` ends line `k`."""
    style, chomp = header.group(1), header.group(3) or header.group(4) or ""
    step = header.group(2) or header.group(5)
    first = k + 1
    while first < len(lines) and not lines[first].strip(" "):
        first += 1
    auto = indent_of(lines[first]) if first < len(lines) else 0
    n = parent + int(step) if step else max(auto, parent + 1)
    if any(len(line) > n for line in lines[k + 1 : first]):
        raise Refused(f"line {k}: a leading empty line deeper than its block")
    body, trailing, k = [], 0, k + 1
    while k < len(lines) and (not lines[k].strip(" ") or indent_of(lines[k]) >= n):
        body.append(lines[k][n:])
        k += 1
    while body and not body[-1]:
        body.pop()
        trailing += 1
    kept = "\n" * trailing if chomp == "+" else ""
    if not body:
        return kept, k
    if style == ">":
        if any(x[:1] in (" ", "\t") for x in body):
            raise Refused(f"line {k}: a more-indented line in a folded block")
        text, empty = "", 0
        for x in body:
            if not x:
                empty += 1
                continue
            text += ("\n" * empty or " ") if text else "\n" * empty
            text, empty = text + x, 0
    else:
        text = "\n".join(body)
    return (text, k) if chomp == "-" else (text + "\n" + kept, k)


def yaml_entries(text):
    """[(key path, value)] for every key of a workflow's block mappings: a scalar decoded by YAML's
    rules, a one-line flow list as its items, and None for a nested block. Anything else (a flow
    mapping, an anchor, an alias, a tag, a quoted or complex key, a directive) is refused."""
    if "\r" in text or "\ufeff" in text:
        raise Refused("a carriage return or a byte-order mark")
    lines = text.split("\n")
    entries, stack, k = [], [], 0
    while k < len(lines):
        line = lines[k]
        body = line.lstrip(" ")
        if not body.strip(" \t") or body.startswith("#"):
            k += 1
            continue
        col = indent_of(line)
        if body[0] == "\t" or (col == 0 and body.startswith(("%", "---", "..."))):
            raise Refused(f"line {k}: a tab indent, a directive or a document marker")
        while body == "-" or body.startswith("- "):
            while stack and (stack[-1][0] > col or (stack[-1][0] == col and not stack[-1][2])):
                stack.pop()
            rest = body[1:].lstrip(" ")
            col, body = col + len(body) - len(rest), rest
        if not body:
            k += 1
            continue
        key = YAML_KEY.match(body)
        if not key:
            if body[0] in "'\"":
                _, k, i = flow_scalar(lines, k, col, col - 1)
                after = lines[k][i:].strip(" \t")
            elif body[0] == "[" and FLOW_LIST.match(body):
                after = ""
            elif body[0] in "[{&*!|>?%@`" or re.search(r":(?:[ \t]|$)", body.split(" #")[0]):
                after = ":"
            else:
                after = ""
            if after and not after.startswith("#"):
                raise Refused(f"line {k}: a key or a node this reader does not read")
            k += 1
            continue
        while stack and stack[-1][0] >= col:
            stack.pop()
        name, value = key.group(1), body[key.end() :]
        path = tuple(entry[1] for entry in stack) + (name,)
        k, decoded = yaml_value(lines, k, col, value)
        stack.append((col, name, decoded is None))
        entries.append((path, decoded))
    return entries


def yaml_value(lines, k, col, value):
    """(next line, decoded value) of the value `value` that follows a key at column `col`."""
    if not value or value.startswith("#"):
        nxt = next(
            (x for x in lines[k + 1 :] if x.strip(" \t") and not x.lstrip().startswith("#")), ""
        )
        if indent_of(nxt) > col and not (
            YAML_KEY.match(nxt.lstrip(" ")) or nxt.lstrip(" ")[:2] == "- "
        ):
            raise Refused(f"line {k}: a scalar on the line after its key")
        return k + 1, None
    header = BLOCK_HEADER.match(value)
    if header:
        text, k = block_scalar(lines, k, header, col)
        return k, text
    if value[0] in "'\"":
        text, k, i = flow_scalar(lines, k, len(lines[k]) - len(value), col)
        if lines[k][i:].strip(" \t") and not lines[k][i:].strip(" \t").startswith("#"):
            raise Refused(f"line {k}: text after a quoted scalar")
        return k + 1, text
    if value[0] == "[":
        items = FLOW_LIST.match(value)
        if not items:
            raise Refused(f"line {k}: a flow list this reader does not read")
        return k + 1, [item.strip() for item in items.group(1).split(",") if item.strip()]
    if value[0] in "{&*!|>?%@`":
        raise Refused(f"line {k}: a flow mapping, an anchor, an alias or a tag")
    # A comment ends a plain scalar, on its first line or a later one.
    comment = re.search(r"[ \t]#", value)
    text, parts, k = value[: comment.start() if comment else None].rstrip(" \t"), [], k + 1
    while not comment and k < len(lines):
        part = lines[k].strip(" \t")
        if part and (indent_of(lines[k]) <= col or part.startswith("#")):
            break
        comment = re.search(r"[ \t]#", part)
        parts.append(part[: comment.start() if comment else None].rstrip(" \t"))
        k += 1
    after = next((x for x in lines[k:] if x.strip(" \t")), "")
    if indent_of(after) > col and not after.lstrip(" \t").startswith("#"):
        raise Refused(f"line {k}: text after a comment that ends a plain scalar")
    while parts and not parts[-1]:
        parts.pop()
    if any(": " in part or ":\t" in part for part in parts):
        raise Refused(f"line {k}: a key inside a plain scalar")
    empty = 0
    for part in parts:
        if not part:
            empty += 1
            continue
        text, empty = text + ("\n" * empty or " ") + part, 0
    if ": " in text or text.endswith(":"):
        raise Refused(f"line {k}: a plain scalar that holds a key")
    return k, text


EXPRESSION = re.compile(r"\$\{\{(.*?)\}\}", re.DOTALL)


def run_texts(text):
    """Each text GitHub can hand bash from a workflow's `run:` values. A `${{ matrix.<key> }}`
    takes each value its job lists; any other expression's value is not in the workflow, and a
    value can end one command and start another (GitHub's hardening guide), so it is refused."""
    entries = yaml_entries(text)
    for path, value in entries:
        if path[-1] == "shell" and value != "bash":
            raise Refused(f"the shell {value!r}, which is not bash")
    for path, value in entries:
        if path[-1] != "run" or not isinstance(value, str):
            continue
        job = path[:2] if path[:1] == ("jobs",) else None
        lists = {
            p[-1]: v
            for p, v in entries
            if job and p[:2] == job and "matrix" in p and isinstance(v, list)
        }
        if any(p[:2] == job and {"include", "exclude"} & set(p) for p, _ in entries):
            lists = {}
        variants = [value]
        for expression in dict.fromkeys(EXPRESSION.findall(value)):
            name = re.fullmatch(r"\s*matrix\.([A-Za-z0-9_-]+)\s*", expression)
            if not name or name.group(1) not in lists:
                raise Refused(f"the expression ${{{{{expression}}}}}, whose value is not stated")
            spelled = "${{" + expression + "}}"
            variants = [v.replace(spelled, item) for v in variants for item in lists[name.group(1)]]
        for variant in variants:
            if "${{" in variant:
                raise Refused("an unclosed expression")
            yield variant


SEPARATORS = {";", ";;", ";&", ";;&", "&&", "||", "|", "|&", "&", "(", ")", "\n"}
REDIRECTS = {"<", ">", ">>", ">|", "<>", "<&", ">&", "&>", "&>>", "<<", "<<-", "<<<"}
OPERATORS = sorted(SEPARATORS - {"\n"} | REDIRECTS, key=len, reverse=True)
ANSI_C = dict(zip("abeEfnrtv\\'\"?", "\a\b\x1b\x1b\f\n\r\t\v\\'\"?"))
PARAMETER = re.compile(r"[#!]?(?:[A-Za-z_][A-Za-z0-9_]*|[0-9]+|[@*#?$!-])")
ARITHMETIC = set("0123456789_ \t\n+-*/%<>=!&|^~?:,#") | set(string.ascii_letters)
SHELL_STATE = {"shopt", "enable", "set", "alias"}
ASSIGNMENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]*\])?\+?=")
# The words after which bash runs the next word as a program, with the words after it unchanged.
LEADERS = {"if", "then", "else", "elif", "do", "while", "until", "{", "!", "time", "command"}
LEADERS |= {"builtin", "exec"}
# The programs whose text is Python, which this guard does not read (SPEC-129 section 8).
PYTHON = ("python3", "python")

# The builtin that reads its arguments as shell, and the shells that read the text after `-c`.
EVALUATORS = ("eval",)
SHELLS = ("bash", "sh")
# The characters that make bash read a text as other than the one plain word it spells.
SPECIAL = re.compile(r"[\s'\"\\$`;&|()<>#*?\[\]{}~=!\0]")
# The `set` options that change what bash reads from text or passes as arguments.
SET_READING = {"keyword", "posix", "histexpand"}


class Word:
    """One word as bash reads it: `value` after quote removal (a NUL where an expansion goes),
    `raw` as written, `dynamic` when an expansion decides it, `io` for a redirection's number,
    and `texts`, what it hands on: its value, an assignment's value, or a here-document's body."""

    def __init__(self, value, raw, dynamic, io, texts=None, bare=False):
        self.value, self.raw, self.dynamic, self.io = value, raw, dynamic, io
        self.texts = [] if texts is None else texts
        self.bare = bare  # an expansion stands outside double quotes, where bash splits its result

    def can_be_dashes(self):
        """Whether an expansion can leave the word as exactly `--`: bash splits an unquoted one into
        words of its own, and a word with no literal character besides `-` is whatever it expands to."""
        return "\0" in self.value and (self.bare or all(c in "\0-" for c in self.value))

    def shown(self):
        text = self.raw if self.dynamic else self.value
        return text.replace(" ", "\\x20").replace("\t", "\\t").replace("\n", "\\n")


class Shell:
    """Bash's reading of one script: its simple commands (in every substitution too) as words,
    and `data`, the texts that can spell `cargo` or `mutants` and that it hands on (a word's or an
    assignment's value, an expansion's operand, a here-document or a here-string, except a Python
    program's), which bash or another program may read as shell."""

    def __init__(self, text):
        self.text, self.i = text, 0
        self.commands, self.data, self.heredocs = [], [], []

    def refuse(self, why):
        raise Refused(f"{why}, at {self.text[max(0, self.i - 20) : self.i + 20]!r}")

    def at(self, k=0):
        return self.text[self.i + k : self.i + k + 1]

    def read(self, closer=False):
        """Read commands to the end, or to a substitution's unmatched `)`."""
        tokens, depth, pending = [], 0, len(self.heredocs)
        while True:
            while self.at() in (" ", "\t") or (self.at() == "\\" and self.at(1) == "\n"):
                self.i += 2 if self.at() == "\\" else 1
            c = self.at()
            if not c:
                if closer:
                    self.refuse("an unclosed substitution")
                break
            if c == "#":
                end = self.text.find("\n", self.i)
                self.i = len(self.text) if end < 0 else end
            elif c == "\n":
                self.i += 1
                tokens.append("\n")
                self.here_documents()
            elif c == ")" and closer and depth == 0:
                if len(self.heredocs) > pending:
                    self.refuse("a here-document inside a substitution's one line")
                self.i += 1
                break
            elif self.text.startswith(("<(", ">("), self.i):
                tokens.append(self.word())
            elif self.text.startswith("((", self.i):
                self.i += 2
                self.arithmetic()
            elif c in ";&|()<>":
                op = next(op for op in OPERATORS if self.text.startswith(op, self.i))
                self.i += len(op)
                depth += {"(": 1, ")": -1}.get(op, 0) if closer else 0
                tokens.append(op)
                if op in ("<<", "<<-"):
                    body = Word("", "", False, True)
                    self.here_document(op, body)
                    tokens.append(body)
            else:
                word = self.word()
                if not word.dynamic and (word.value == "=~" or (closer and word.value == "case")):
                    self.refuse("a regular expression or a case inside a substitution")
                tokens.append(word)
        words, texts, target = [], [], False
        for token in tokens + ["\n"]:
            if isinstance(token, str):
                target = token in REDIRECTS
                if token in SEPARATORS and (words or texts):
                    self.command(words, texts)
                    words, texts = [], []
                continue
            texts += token.texts
            if target or token.io:
                target = False
            else:
                words.append(token)

    def command(self, words, texts):
        program = next((w for w in words if not ASSIGNMENT.match(w.value)), None)
        python = program and not program.dynamic and program.value.rsplit("/", 1)[-1] in PYTHON
        # A python command that could run the memory scope's wrapper runs a command, not Python:
        # one with an argument that names the wrapper, or that bash computes. Its texts are read.
        arguments = words[words.index(program) + 1 :] if python else []
        if not python or any(w.dynamic or "memory_scope" in w.value for w in arguments):
            self.data += texts
        if not words:
            return
        k = 0
        while k + 1 < len(words) and words[k].value in ("builtin", "command"):
            k += 1
        head, rest = words[k], words[k + 1 :]
        if head.value in SHELL_STATE and not head.dynamic:
            options = [w for w in rest if w.value[:1] in "-+" and w.value != "--"]
            named = [
                rest[n + 1] for n, w in enumerate(rest[:-1]) if w in options and "o" in w.value
            ]
            if (
                head.value != "set"
                or any(w.dynamic or set("kH") & set(w.value) for w in options)
                or any(w.dynamic or w.value in SET_READING for w in named)
            ):
                self.refuse(f"`{head.value}`, which changes how bash reads what follows")
        self.commands.append(words)

    def word(self, element=False):
        start, value, dynamic, bare = self.i, [], False, False
        if self.text.startswith(("<(", ">("), self.i):
            self.i += 2
            self.read(closer=True)
            value, dynamic, bare = ["\0"], True, True
        while True:
            c = self.at()
            if not c or c in " \t\n;&|<>)":
                if c in "<>" and self.at(1) == "(" and self.i > start:
                    self.refuse("a process substitution glued to a word")
                break
            if c == "(":
                if self.i > start and self.text[self.i - 1] in "@*+?!":
                    # A pattern character glued to `(` is one word to bash where extended globbing
                    # is on, as it always is after `==`, `!=` and `=` in `[[ ]]`.
                    self.refuse("an extended pattern, which bash reads as one word")
                if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*\+?=", self.text[start : self.i]):
                    break
                self.array()
                value, dynamic, bare = value + ["\0"], True, True
            elif c == "\\":
                if self.at(1) != "\n":
                    value.append(self.at(1) or "\\")
                self.i += 2
            elif c == "'":
                end = self.text.find("'", self.i + 1)
                if end < 0:
                    self.refuse("an unclosed single quote")
                value.append(self.text[self.i + 1 : end])
                self.i = end + 1
            elif c == '"':
                dynamic = self.double_quoted(value) or dynamic
            elif c == "$":
                expansions = value.count("\0")
                dynamic = self.dollar(value, quoted=False) or dynamic
                bare = bare or value.count("\0") > expansions
            elif c == "`":
                self.backquote()
                value, dynamic, bare = value + ["\0"], True, True
            elif c == "[" and (
                # bash reads a subscript as one word after a name, and at an array element's start
                re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", self.text[start : self.i])
                or (element and self.i == start)
            ):
                end = self.text.find("]", self.i)
                if end < 0 or not re.fullmatch(r"[A-Za-z0-9_$+*/%-]*", self.text[self.i + 1 : end]):
                    self.refuse("a subscript")
                value.append(self.text[self.i : end + 1])
                self.i, dynamic = end + 1, True
            else:
                glob = (
                    c in "*?[" or (c in "{}" and self.i > start) or (c == "~" and self.i == start)
                )
                glob = glob or (c == "{" and self.at(1) not in ("", " ", "\t", "\n"))
                value.append(c)
                self.i, dynamic = self.i + 1, dynamic or glob
        raw, text = self.text[start : self.i], "".join(value)
        io = self.at() in ("<", ">") and bool(
            re.fullmatch(r"[0-9]+|\{[A-Za-z_][A-Za-z0-9_]*\}", raw)
        )
        # A value is handed on where reading it again can give other words than this one word; an
        # assignment's value is handed on whole, as `$name` gives it back.
        assignment = ASSIGNMENT.match(text)
        texts = [text] if SPECIAL.search(text) else []
        texts = [t for t in texts + [text[assignment.end() :] if assignment else ""] if suspect(t)]
        return Word(text, raw, dynamic or "\0" in text, io, texts, bare)

    def double_quoted(self, value):
        self.i += 1
        dynamic = False
        while True:
            c = self.at()
            if not c:
                self.refuse("an unclosed double quote")
            if c == '"':
                self.i += 1
                return dynamic
            if c == "\\":
                if self.at(1) in ("$", "`", '"', "\\"):
                    value.append(self.at(1))
                elif self.at(1) != "\n":
                    value.append("\\")
                    self.i -= 1
                self.i += 2
            elif c == "$" and self.at(1) not in ("'", '"'):
                dynamic = self.dollar(value, quoted=True) or dynamic
            elif c == "`":
                self.backquote()
                value.append("\0")
                dynamic = True
            else:
                value.append(c)
                self.i += 1

    def dollar(self, value, quoted):
        """Read the `$` form at the cursor into `value`; True when an expansion decides it."""
        nxt = self.at(1)
        if nxt == "'" and not quoted:
            self.i += 2
            value.append(self.ansi_c())
            return False
        if nxt == '"' and not quoted:
            self.i += 1
            return self.double_quoted(value)
        value.append("\0")
        if self.text.startswith("$((", self.i):
            self.i += 3
            self.arithmetic()
        elif nxt == "(":
            self.i += 2
            self.read(closer=True)
        elif nxt == "{":
            self.parameter(quoted)
        elif nxt == "[":
            self.refuse("`$[`, an arithmetic expansion bash reads as one word")
        elif nxt and (nxt.isalnum() or nxt in "_@*#?$!-"):
            name = PARAMETER.match(self.text, self.i + 1)
            self.i = name.end() if nxt.isalpha() or nxt == "_" else self.i + 2
        else:
            value[-1] = "$"
            self.i += 1
            return False
        return True

    def ansi_c(self):
        out = []
        while True:
            c = self.at()
            if not c:
                self.refuse("an unclosed ANSI-C string")
            if c == "'":
                self.i += 1
                return "".join(out).split("\0")[0]
            if c != "\\":
                out.append(c)
                self.i += 1
                continue
            d = self.at(1)
            octal = re.match(r"[0-7]{1,3}", self.text[self.i + 1 : self.i + 4])
            sized = {"x": 2, "u": 4, "U": 8}.get(d)
            digits = re.match(r"[0-9a-fA-F]+", self.text[self.i + 2 : self.i + 2 + (sized or 0)])
            if d in ANSI_C:
                out.append(ANSI_C[d])
                self.i += 2
            elif octal:
                out.append(chr(int(octal.group(0), 8) & 0xFF))
                self.i += 1 + len(octal.group(0))
            elif sized and digits:
                out.append(chr(int(digits.group(0), 16)))
                self.i += 2 + len(digits.group(0))
            elif d == "c" and self.at(2):
                out.append(chr(ord(self.at(2)) & 0x1F))
                self.i += 3
            else:
                out.append("\\")
                self.i += 1

    def parameter(self, quoted):
        self.i += 2
        name = PARAMETER.match(self.text, self.i)
        if not name:
            self.refuse("a parameter expansion this reader does not read")
        self.i = start = name.end()
        while True:
            c = self.at()
            if not c or c == "{" or (c == "'" and quoted) or c == "[":
                self.refuse("a parameter expansion this reader does not read")
            if c == "}":
                if suspect(self.text[start : self.i]):
                    self.data.append(self.text[start : self.i])
                self.i += 1
                return
            if c == "\\":
                self.i += 2
            elif c == "'":
                end = self.text.find("'", self.i + 1)
                if end < 0:
                    self.refuse("an unclosed single quote")
                self.i = end + 1
            elif c == '"':
                self.double_quoted([])
            elif c == "$":
                self.dollar([], quoted)
            elif c == "`":
                self.backquote()
            else:
                self.i += 1

    def arithmetic(self):
        depth = 0
        while True:
            c = self.at()
            if c == ")" and depth == 0:
                if self.at(1) != ")":
                    self.refuse("an arithmetic expansion this reader does not read")
                self.i += 2
                return
            if c == "$":
                self.dollar([], quoted=True)
                continue
            if c not in ARITHMETIC | {"(", ")"} or not c:
                self.refuse("an arithmetic expansion this reader does not read")
            depth += {"(": 1, ")": -1}.get(c, 0)
            self.i += 1

    def array(self):
        self.i += 1
        while True:
            while self.at() in (" ", "\t", "\n") or (self.at() == "\\" and self.at(1) == "\n"):
                self.i += 2 if self.at() == "\\" else 1
            c = self.at()
            if c == ")":
                self.i += 1
                return
            if c == "#":
                end = self.text.find("\n", self.i)
                self.i = len(self.text) if end < 0 else end
            elif not c or c in ";&|(<>":
                self.refuse("an array this reader does not read")
            else:
                self.word(element=True)

    def backquote(self):
        end = self.text.find("`", self.i + 1)
        if end < 0 or "\\" in self.text[self.i : end]:
            self.refuse("a backquote this reader does not read")
        inner = Shell(self.text[self.i + 1 : end])
        inner.read()
        self.commands += inner.commands
        self.data += inner.data
        self.i = end + 1

    def here_document(self, op, body):
        while self.at() in (" ", "\t"):
            self.i += 1
        word = self.word()
        if word.dynamic or not word.raw:
            self.refuse("a here-document this reader does not read")
        self.heredocs.append((word.value, word.raw != word.value, op == "<<-", body))

    def here_documents(self):
        """Read each body the line opened, to its delimiter's line or to the end of the text, as
        the program it goes to reads it: an unquoted delimiter's body with each expansion a NUL."""
        while self.heredocs:
            delimiter, quoted, tabs, body = self.heredocs.pop(0)
            out = []
            while self.i < len(self.text):
                end = self.text.find("\n", self.i)
                end = len(self.text) if end < 0 else end
                while tabs and self.at() == "\t":
                    self.i += 1
                if self.text[self.i : end] == delimiter:
                    self.i = min(end + 1, len(self.text))
                    break
                if quoted:
                    out.append(self.text[self.i : end])
                    self.i = end
                while self.i < end:
                    c = self.at()
                    if c == "\\" and self.i + 1 == end:
                        self.refuse("a continued line in a here-document")
                    if c == "$" and self.at(1) not in ("'", '"'):
                        self.dollar(out, quoted=True)
                    elif c == "`":
                        self.backquote()
                        out.append("\0")
                    elif c == "\\" and self.at(1) in ("$", "`", "\\"):
                        out.append(self.at(1))
                        self.i += 2
                    else:
                        out.append(c)
                        self.i += 1
                if self.i > end:
                    self.refuse("an expansion that runs past its here-document line")
                out.append("\n")
                self.i = end + 1
            text = "".join(out)
            if suspect(text):
                body.texts.append(text)


CARGO = ("cargo", "cargo-mutants")
VALUED_FLAGS = {"--config", "--color", "-C", "-Z", "--explain"}
PLAIN_FLAGS = {
    "--version",
    "--list",
    "--verbose",
    "--quiet",
    "--locked",
    "--offline",
    "--frozen",
    "--help",
}


def mutants_of(words, handed=False, wrapped=no_wrapper):
    """The `cargo mutants` commands among one simple command's words, each shown from its program
    word; the words after a `--` go to the test tool, so they are shown joined and bound nothing.
    A command is found only where bash itself runs `cargo` as the program, or where the wrapper's
    form runs it as the first word after its `--` (`wrapped`). Refused: a literal
    `cargo` word whose subcommand bash computes or never gives; `cargo mutants` anywhere else, or
    in a text another program runs (`handed`), where that program decides its arguments; and a
    literal `mutants` word that is not a found command's subcommand. A text bash computes for a
    program that reads it as shell (`EVALUATORS`, or `-c` to one of `SHELLS`) is refused."""
    program = 0
    while program < len(words) and (
        ASSIGNMENT.match(words[program].value) or words[program].value in LEADERS
    ):
        program += 1
    # The wrapper runs its command without a shell: its first word is the program, as it stands.
    while program < len(words) and (after := wrapped(words, program)) is not None:
        program = after
    name = words[program].value.rsplit("/", 1)[-1] if program < len(words) else ""
    rest = words[program + 1 :]
    if name in EVALUATORS or (
        name in SHELLS and any(re.fullmatch(r"-[a-z]*c[a-z]*", w.value) for w in rest)
    ):
        text = rest if name in EVALUATORS else [w for w in rest if w.value[:1] not in "-+"][:1]
        upto = rest.index(text[-1]) + 1 if text else len(rest)
        if any(w.dynamic for w in rest[:upto]):
            raise Refused(f"a text bash computes for `{words[program].raw}` to read as shell")
    found, used = [], set()
    for p, word in enumerate(words):
        if word.dynamic or word.value.rsplit("/", 1)[-1] not in CARGO:
            continue
        q, valued = p + 1, False
        while q < len(words):
            w = words[q]
            if w.dynamic:
                raise Refused(f"a word bash computes where cargo reads its subcommand: {w.raw}")
            if w.value == "mutants" and (p != program or handed):
                raise Refused(f"`{word.raw} mutants` where another program decides its arguments")
            if w.value == "mutants":
                args = [x.shown() for x in words[p:]]
                cut = next((k for k in range(q - p, len(args)) if args[k] == "--"), len(args))
                # R5: a word before the bounds that bash can expand to exactly `--` ends cargo's
                # options there, so the bounds after it are the test tool's, and the guard cannot
                # tell from the text whether it does.
                bounds = BOUNDS.split()
                where = next(
                    (
                        k
                        for k in range(q - p, len(args))
                        if args[k : k + len(bounds)] == bounds and k < cut
                    ),
                    len(args),
                )
                for early in words[q + 1 : p + where]:
                    if early.can_be_dashes():
                        raise Refused(f"a word bash computes before the bounds: {early.raw}")
                found.append(
                    " ".join(args[:cut] + (["\\x20".join(args[cut:])] if args[cut:] else []))
                )
                used.add(q)
                break
            if w.value[:1] in "+-":
                valued = w.value in VALUED_FLAGS or (
                    w.value[:2] == "--"
                    and "=" not in w.value
                    and w.value not in PLAIN_FLAGS | {"--"}
                )
            elif valued:
                valued = False
            else:
                break
            q += 1
        else:
            raise Refused(f"`{word.raw}` whose subcommand comes from its input or another program")
    for q, word in enumerate(words):
        if word.value == "mutants" and q not in used:
            raise Refused(f"`mutants` that is not the subcommand of a cargo bash runs: {word.raw}")
    return found


def unquoted(text):
    """The text with every quote mark, backslash, `$` and line break gone: a superset of what
    quote removal, in the text and in every text it hands on, can spell from it."""
    return re.sub(r"['\"\\$\n]", "", text)


def suspect(text):
    """Whether quote removal, here or in a text handed on, can spell `mutants` or `cargo` from
    this text, or an ANSI-C string can (a `$` and a `'` with only quoting between them)."""
    bare = unquoted(text)
    return "mutants" in bare or "cargo" in bare or bool(re.search(r"\$['\"\\$\n]*'", text))


def mutants_in(text, handed=False, wrapped=no_wrapper):
    """The `cargo mutants` commands bash would run from one script, as words; each text it hands
    on is read again by the same grammar, and any command there is refused. A text that cannot
    spell `cargo` or `mutants` is not read."""
    if not suspect(text):
        return []
    shell = Shell(text)
    shell.read()
    found = [line for words in shell.commands for line in mutants_of(words, handed, wrapped)]
    for data in shell.data:
        if data != text:
            mutants_in(data, handed=True, wrapped=wrapped)
    return found
