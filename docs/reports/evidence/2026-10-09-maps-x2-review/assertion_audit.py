"""Review changed Rust test functions using maintained tree-sitter grammars.

Normal Rust functions come from the grammar's function_item nodes. Proptest's
DSL signatures are token trees; the grammar parses their brace bodies as Rust
blocks. Argument groups are grammar token_tree nodes, so nested commas are not
mistaken for assertion-argument separators. This is review tooling, not a gate.
"""
from pathlib import Path
import subprocess
import json
import sys
from tree_sitter import Language, Parser
import tree_sitter_rust

ROOT = Path(sys.argv[1])
OUT = Path(sys.argv[2])
PARSER = Parser(Language(tree_sitter_rust.language()))
ASSERTS = {'assert', 'assert_eq', 'assert_ne', 'prop_assert', 'prop_assert_eq', 'prop_assert_ne'}

def git(*args):
    return subprocess.check_output(['git', '-C', str(ROOT), *args], stderr=subprocess.DEVNULL)

def nodes(node):
    yield node
    for child in node.children:
        yield from nodes(child)

def text(node, source):
    return source[node.start_byte:node.end_byte].decode()

def attributes(node, source):
    parts = []
    previous = node.prev_named_sibling
    while previous and previous.type == 'attribute_item':
        parts.append(text(previous, source))
        previous = previous.prev_named_sibling
    return '\n'.join(parts)

def test_scope(node, source, file):
    if '/tests/' in file or file.endswith('/tests.rs'):
        return True
    at = node
    while at:
        if '#[test]' in attributes(at, source) or '#[cfg(test)]' in attributes(at, source):
            return True
        at = at.parent
    return False

def functions(source, file):
    tree = PARSER.parse(source)
    found = {}
    for node in nodes(tree.root_node):
        if node.type == 'function_item' and test_scope(node, source, file):
            name = text(node.child_by_field_name('name'), source)
            modules = []
            at = node.parent
            while at:
                if at.type == 'mod_item':
                    modules.append(text(at.child_by_field_name('name'), source))
                at = at.parent
            key = '::'.join([*reversed(modules), name])
            found[key] = (source[node.start_byte:node.end_byte], node.start_point.row + 1, False)
        if node.type != 'macro_invocation':
            continue
        macro_name = text(node.child_by_field_name('macro'), source).split('::')[-1]
        if macro_name != 'proptest':
            continue
        tokens = next(child for child in node.children if child.type == 'token_tree').children
        for index, token in enumerate(tokens):
            if token.type != 'fn':
                continue
            name = text(tokens[index + 1], source)
            body = next((candidate for candidate in tokens[index + 2:] if candidate.type == 'token_tree' and text(candidate, source).startswith('{')), None)
            if body:
                body_source = source[body.start_byte:body.end_byte]
                found['proptest::' + name] = (b'fn review_body() ' + body_source, body.start_point.row + 1, True)
    return found

def arguments(token_tree, source):
    args = []
    start = token_tree.start_byte + 1
    for child in token_tree.children[1:-1]:
        if child.type == ',':
            args.append(source[start:child.start_byte].decode().strip())
            start = child.end_byte
    last = source[start:token_tree.end_byte - 1].decode().strip()
    if last:
        args.append(last)
    return args

def observation_kind(argument):
    expression = PARSER.parse(('fn expression() { let observed = ' + argument + '; }').encode())
    declaration = next((node for node in nodes(expression.root_node) if node.type == 'let_declaration'), None)
    if not declaration:
        return None
    value = declaration.child_by_field_name('value')
    if value and value.type == 'tuple_expression':
        return 'tuple'
    if value and value.type == 'array_expression':
        if any(child.type == 'boolean_literal' for child in value.named_children):
            return 'boolean-vector'
        if any(child.type == 'tuple_expression' for child in value.named_children):
            return 'tuple-vector'
    if value and value.type == 'macro_invocation':
        macro = text(value.child_by_field_name('macro'), ('fn expression() { let observed = ' + argument + '; }').encode()).split('::')[-1]
        if macro == 'vec':
            fragment = ('fn expression() { let observed = ' + argument + '; }').encode()
            group = next(child for child in value.children if child.type == 'token_tree')
            if any(child.type == 'boolean_literal' for child in group.named_children):
                return 'boolean-vector'
            for item in arguments(group, fragment):
                item_tree = PARSER.parse(('fn item() { let observed = ' + item + '; }').encode())
                item_declaration = next((node for node in nodes(item_tree.root_node) if node.type == 'let_declaration'), None)
                item_value = item_declaration.child_by_field_name('value') if item_declaration else None
                if item_value and item_value.type == 'tuple_expression':
                    return 'tuple-vector'
    return None

def audit(revision):
    files = git('diff', '--name-only', 'origin/lane/claude/MAPS-X1', revision).decode().splitlines()
    report = {'revision': revision, 'test_functions_changed': 0, 'assertions': 0, 'missing_message': [], 'tuple_or_boolean_vector': []}
    for file in files:
        if not file.endswith('.rs'):
            continue
        try:
            source = git('show', revision + ':' + file)
        except subprocess.CalledProcessError:
            continue
        try:
            before = functions(git('show', 'origin/lane/claude/MAPS-X1:' + file), file)
        except subprocess.CalledProcessError:
            before = {}
        for name, (body, start_line, synthetic) in functions(source, file).items():
            if name in before and before[name][0] == body:
                continue
            report['test_functions_changed'] += 1
            body_tree = PARSER.parse(body)
            for node in nodes(body_tree.root_node):
                if node.type != 'macro_invocation':
                    continue
                macro = text(node.child_by_field_name('macro'), body).split('::')[-1]
                if macro not in ASSERTS:
                    continue
                group = next(child for child in node.children if child.type == 'token_tree')
                args = arguments(group, body)
                report['assertions'] += 1
                site = {'file': file, 'function': name, 'line': start_line + node.start_point.row, 'assertion': text(node, body)}
                required = 2 if macro.endswith('_eq') or macro.endswith('_ne') else 1
                if len(args) <= required:
                    report['missing_message'].append(site)
                for argument in args[:required]:
                    kind = observation_kind(argument)
                    if kind:
                        report['tuple_or_boolean_vector'].append({**site, 'candidate': kind})
                        break
    return report

reports = [audit(sys.argv[3])]
OUT.write_text(json.dumps(reports, indent=2) + '\n')
for report in reports:
    print(json.dumps({key: len(value) if isinstance(value, list) else value for key, value in report.items()}))
for report in reports:
    if report['revision'] == sys.argv[3]:
        for category in ['missing_message', 'tuple_or_boolean_vector']:
            for site in report[category]:
                print(category, json.dumps(site))
