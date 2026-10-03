import re

path = "/Users/ahs/Project/Rust/floem-ui/crates/floem-elements/Cargo.toml"
text = open(path).read()

# Collect all cupertino + fluent optional dependency names declared in [dependencies].
deps = re.findall(r'^(floem-widget-(?:cupertino|fluent)-[a-z0-9_]+)\s*=', text, re.M)

# Locate the `multi = [ ... ]` block.
m = re.search(r'multi\s*=\s*\[(.*?)\]', text, re.S)
assert m, "multi feature block not found"
block = m.group(1)
existing = set(re.findall(r'"(floem-widget-[a-z0-9_-]+)"', block))

to_add = [d for d in deps if d not in existing]
new_entries = ',\n'.join(f'    "{d}"' for d in to_add)

# Rebuild the block contents, preserving existing entries + formatting.
stripped = block.rstrip().rstrip(',')
new_block = stripped
if new_block.strip():
    new_block += ',\n'
new_block += new_entries + '\n'

text2 = text[:m.start(1)] + new_block + text[m.end(1):]
open(path, 'w').write(text2)
print(f"added {len(to_add)} crates to multi feature")
print("first few:", to_add[:5])
