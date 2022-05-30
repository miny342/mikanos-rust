import pprint

mouse = [
    "@           ",
    "@@          ",
    "@.@         ",
    "@..@        ",
    "@...@       ",
    "@....@      ",
    "@.....@     ",
    "@......@    ",
    "@.......@   ",
    "@........@  ",
    "@.........@ ",
    "@.....@@@@@@",
    "@..@@@      ",
    "@@@         ",
]

output = []

for s in mouse:
    op = [0, 0, 0]
    for i, v in enumerate(s):
        idx = i // 4
        op[idx] = op[idx] << 2 | (0 if v == " " else (1 if v == "@" else 2))
    output.append(op)

pprint.pprint(output)
