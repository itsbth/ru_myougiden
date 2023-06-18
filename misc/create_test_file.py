import gzip

IN_FILE = "JMdict_e.gz"
OUT_FILE = "testdata/JMdict_e_test.gz"
NUM_ENTRIES = 100

# known entry for testing
AKASABI_ENTRY = """
<entry>
<ent_seq>1829380</ent_seq>
<k_ele>
<keb>赤錆</keb>
</k_ele>
<k_ele>
<keb>赤さび</keb>
</k_ele>
<r_ele>
<reb>あかさび</reb>
</r_ele>
<sense>
<pos>&n;</pos>
<gloss>rust</gloss>
</sense>
</entry>
""".encode(
    "utf-8"
)

entry_count = 0

# Open the JMdict_e.gz file and extract the XML data
with gzip.open(IN_FILE, "rb") as f, gzip.open(OUT_FILE, "wb") as g:
    for line in f:
        g.write(line)
        if line == b"</entry>\n":
            entry_count += 1
            if entry_count == NUM_ENTRIES:
                break
    g.write(AKASABI_ENTRY)
    g.write(b"</JMdict>\n")
