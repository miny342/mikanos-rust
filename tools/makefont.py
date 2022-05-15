import pprint

s = input()

pprint.pprint(list(map(lambda x: list(map(lambda y: int(y.replace('.', '0').replace('@', '1'), 2) ,x.split('\n')[1:17])), s.split('0x')[1:])))
