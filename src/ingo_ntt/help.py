import sys

NTT_WORD_BYTES = 32
infile = sys.argv[1]
outfile = sys.argv[2]
out = open(outfile, 'wb')

with open(infile, 'r') as vector:
    for line in vector:
        element = int(line)                            
        out.write(element.to_bytes(NTT_WORD_BYTES,'little'))