import gzip

IN_FILE = "JMdict_e.gz"
OUT_FILE = "JMdict_e_test.gz"
NUM_ENTRIES = 100

entry_count = 0

# Open the JMdict_e.gz file and extract the XML data
with gzip.open(IN_FILE, "rb") as f, gzip.open(OUT_FILE, "wb") as g:
    for line in f:
        g.write(line)
        if line == b"</entry>\n":
            entry_count += 1
            if entry_count == NUM_ENTRIES:
                break
    g.write(b"</JMdict>\n")
