from json import load
from string import ascii_uppercase
import numpy as np
import re
from functools import reduce
from time import perf_counter

t0 = perf_counter()

Charmask = np.int32
WORD_LENGTH = 5
N_SOLUTIONS = 2315
VALID_WORDS_IDX = -1
VALID_SOLUTIONS_IDX = 0

char2bit = {c:1<<(ord(c)-ord('A')) for c in ascii_uppercase}
bit2char = {k:v for v,k in char2bit.items()}

def load_dictionary(filename):
    print(f"Loading dictionary at {filename}")
    regex = re.compile(r"^[A-Za-z]{"+f"{WORD_LENGTH}"+r"}$")
    with open(filename, 'r') as file:
        words = [line.strip().upper() for line in file.readlines() if regex.match(line)]
        print(f"Loaded {len(words)} words")
        return words

def bitify_word(word):
    return [char2bit[c] for c in word]

def bitify_words(words):
    return np.array([bitify_word(word) for word in words], dtype=Charmask)


# So this time around we'll play SoA instead of AoS and see how we go
WORDS = load_dictionary("words-kura")
WORDS_B = bitify_words(WORDS)
WORDS_BM = np.bitwise_or.reduce(WORDS_B, 1)
WORDS_B_BM = np.vstack((WORDS_B.T, [WORDS_BM])).T  # Bitmask in last column


def simulate(guess_ids, solution_id):
    required_chars = 0
    banned_chars = np.zeros(WORD_LENGTH, Charmask)
    for guess_id in guess_ids:
        required_chars |= WORDS_BM[guess_id] & WORDS_BM[solution_id]
        banned_chars |= WORDS_BM[guess_id] & ~WORDS_BM[solution_id]
    reqs_slice = (WORDS_BM & required_chars) == required_chars
    return sum((WORDS_B[reqs_slice] & banned_chars).any(1))

def find_worstcase(guess_ids):
    worst_idx = 0
    worst_remaining = 0
    for solution_id in range(0, N_SOLUTIONS):
        remaining = simulate(guess_ids, solution_id)
        if remaining > worst_remaining:
            worst_remaining = remaining
            worst_idx = solution_id
    return worst_remaining, worst_idx

# def _generate_wordcache_nested(cache, subcache, keymask, depth, lastidx):
#     # Guess we'll have subcache as WORDS_BM for now
#     for idx in range(lastidx, 26):
#         ib = 1<<idx
#         sc2 = WORDS_BM[]

# def generate_wordcache(valid_words):
#     valid_solutions = valid_words[:N_SOLUTIONS]

t1 = perf_counter()

worst_per_guess = [find_worstcase([guess]) for guess in range(0, len(WORDS_B))]

t2 = perf_counter()

print(f'Setup time: {t1-t0}    Loop time: {t2-t1}')