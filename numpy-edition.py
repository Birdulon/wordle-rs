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


def _generate_wordcache_nested(cache, subcache, keymask, depth, lastidx):
    # Guess we'll have subcache as WORDS_BM for now
    for idx in range(lastidx, 26):
        ib = 1<<idx
        sc2 = subcache[subcache[:,-1] & keymask == keymask]
        if len(sc2) > 0:
            cache[keymask] = sc2
            km2 = keymask | ib
            if depth > 0:
                _generate_wordcache_nested(cache, sc2, km2, depth-1, idx+1)

def generate_wordcache(valid_words):
    valid_solutions = valid_words[:N_SOLUTIONS]
    cache = {}
    _generate_wordcache_nested(cache, valid_solutions, 0, 5, 0)
    return cache


def simulate(guess_ids):
    # We can merge all of our guesses into a single set of masks
    guess_aggregate = np.bitwise_or.reduce(WORDS_B_BM[guess_ids], 0)
    # We can check our guess contents against all possible solutions
    required_chars = guess_aggregate[-1] & WORDS_BM[:N_SOLUTIONS]
    banned_chars = np.tile(guess_aggregate[-1] & ~WORDS_BM[:N_SOLUTIONS], (5,1)).T
    # Now we need to go through each character position and determine hits and misses
    hits = guess_aggregate[:-1] & WORDS_B[:N_SOLUTIONS]
    banned_chars |= guess_aggregate[:-1] & ~WORDS_B[:N_SOLUTIONS]
    banned_chars[hits > 0] |= ~hits[hits > 0]  # Feels a bit dodge but can't think of anything better

    worst_remaining = 0
    worst_idx = 0
    print('About to loop')
    for sol in range(0, N_SOLUTIONS):
        if required_chars[sol] in CACHE:
            remaining = sum(~(CACHE[required_chars[sol]][:,:-1] & banned_chars[sol,:]).any(1))
            if remaining > worst_remaining:
                worst_remaining = remaining
                worst_idx = sol
    print(f'Completed {guess_ids} of {len(WORDS_B)} - {worst_remaining} words against solution {worst_idx}')
    return worst_remaining, worst_idx

t1 = perf_counter()

CACHE = generate_wordcache(WORDS_B_BM)
print(f'Generated cache with {len(CACHE)} keys')

t2 = perf_counter()

worst_per_guess = [simulate([guess]) for guess in range(0, 1000)]

t3 = perf_counter()

print(f'Setup time: {t1-t0} \tCachegen time: {t2-t1} \tLoop time: {t3-t2}')