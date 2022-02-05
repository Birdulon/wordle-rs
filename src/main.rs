#![allow(dead_code)]
#![allow(unused_imports)]
use std::fs;
use std::collections::{HashMap, BTreeMap};
use bitintr::{Lzcnt, Tzcnt};
use regex::Regex;
use rayon::prelude::*;
use itertools::zip;
use array_init::array_init;

// use ahash::{AHasher, RandomState};
// use xxhash_rust::xxh3::Xxh3;
// use std::hash::BuildHasherDefault;

type Charmask = i32;
type Achar = i8;  // ASCII char

const WORD_LENGTH: usize = 5;
const WORD_LENGTH_P: usize = 5;  // Padded for SIMD shenanigans
const GUESS_DEPTH: usize = 1;  // TODO: Change this whenever working at different depths
const N_SOLUTIONS: usize = 2315;
const CACHE_SIZE: usize = 1<<26;
const IDX_ALL_WORDS: Charmask = (CACHE_SIZE as Charmask) - 1;
const IDX_VALID_SOLUTIONS: Charmask = 0;
const A: Achar = 'A' as Achar;
const Z: Achar = 'Z' as Achar;

#[derive(Copy, Clone, Default)]
struct Word {
    charbits: [Charmask; WORD_LENGTH_P],  // Each letter in bitmask form
    charmask: Charmask,                   // All of the characters contained
    //letters: [Achar; WORD_LENGTH]
}

// type WordCache = HashMap<Charmask, Vec<Word>, RandomState>;  // ahash
// type WordCache = HashMap<Charmask, Vec<Word>, BuildHasherDefault<Xxh3>>;
type WordCache = BTreeMap<Charmask, Vec<Word>>;
// type WordCache = HashMap<Charmask, Vec<Word>>;  // Default hash is slower than BTree
// type WordCacheArr = [&Vec<Word>; CACHE_SIZE];

fn default_wordcache() -> WordCache {
    WordCache::default()
}


fn char2bit(c: Achar) -> Charmask {
    debug_assert!((A..=Z).contains(&c));
    1 << (c - A)
}

fn cm2char(cm: Charmask, offset: i8) -> Achar {
    (((31 - cm.lzcnt() as i8) + A + offset) as u8) as Achar
}

fn letters2str(letters: [Achar; WORD_LENGTH]) -> String {
    letters.iter().map(|x| (*x as u8) as char).collect()
}

fn charbits2str(charbits: [Charmask; WORD_LENGTH]) -> String {
    charbits.iter().map(|x| (cm2char(*x, 0) as u8) as char).collect()
}

fn str2word(s: &str) -> Word {
    let mut word = Word::default();
    let mut iter = s.chars();
    for i in 0..WORD_LENGTH {
        let c = iter.next().unwrap() as Achar;
        let cb = char2bit(c);
        word.charbits[i] = cb;
        //word.letters[i] = c;
        word.charmask |= cb;
    }
    word
}

/* fn inc_char(c: char) -> char {
    (c as u8 + 1) as char
} */

/* fn hs2str(hs: &HashSet<char>) -> String {
    let mut chars: Vec<char> = hs.iter().cloned().collect();
    if chars.is_empty() {
        "".to_string()
    } else {
        chars.sort_unstable();
        chars.iter().collect()
    }
} */

/* fn charmask2str(cm: Charmask) -> String {
    let mut s = String::default();
    for i in cm.tzcnt() ..= 32-cm.lzcnt() {
        if (cm & (1<<i)) != 0 {
            s += &((A + i as Achar) as u8 as char).to_string();
        }
    }
    s
} */

fn load_dictionary(filename: &str) -> Vec<Word> {
    println!("Loading dictionary at {}", filename);
    let rawfile = fs::read_to_string(filename).unwrap();
    let rawwords = rawfile.split('\n');
    let mut words = Vec::new();
    let re = Regex::new(&format!("{}{}{}", r"^[A-Za-z]{", WORD_LENGTH, r"}$")).unwrap();
    for line in rawwords {
        if re.is_match(line) {
            words.push(line.to_uppercase());
        }
    }
    //words.sort();
    //words.dedup();
    words.iter().map(|w| str2word(w)).collect()
}

fn _generate_wordcache_nested(cache: &mut WordCache, subcache: &[Word], key: Charmask, depth: u8) {
    for c in cm2char(key, 1)..=Z {
        let cb = char2bit(c);
        let sc2: Vec<Word> = subcache.iter().filter(|w| (w.charmask & cb) == cb).cloned().collect();
        if !sc2.is_empty() {
            let key2 = key | cb;
            if depth > 0 {
                _generate_wordcache_nested(cache, &sc2, key2, depth-1);
            }
            cache.insert(key2, sc2);
        }
    }
}

fn generate_wordcache(valid_words: Vec<Word>) -> WordCache {
    let mut cache: WordCache = default_wordcache();
    let valid_solutions: Vec<Word> = valid_words[..N_SOLUTIONS].to_vec();  // Hacky way to separate the valid solutions from the larger guessing list
    _generate_wordcache_nested(&mut cache, &valid_solutions, 0, 5);
    cache.insert(IDX_VALID_SOLUTIONS, valid_solutions);
    cache.insert(IDX_ALL_WORDS, valid_words);
    cache
}

fn filter_word(w: &[Charmask; WORD_LENGTH_P], banned_chars: &[Charmask; WORD_LENGTH_P]) -> bool {
    zip(w, banned_chars).all(|(x,y)| x & y == 0)
}

fn aggregate_guesses(guess_ids: Vec<usize>, wordcache: &WordCache) -> Word {
    //guess_ids.iter().reduce(|out, g| out |= wordcache[IDX_ALL_WORDS][g]).unwrap()
    let all_words = &wordcache[&IDX_ALL_WORDS];
    let mut iter = guess_ids.iter();
    let mut aggregate_guess = all_words[*iter.next().unwrap()];
    for g in iter {
        let guess = all_words[*g];
        for i in 0..aggregate_guess.charbits.len() {
            aggregate_guess.charbits[i] |= guess.charbits[i];
        }
        aggregate_guess.charmask |= guess.charmask;
    }
    aggregate_guess
}

fn simulate(guess: Word, wordcache: &WordCache) -> (String, usize) {
    let valid_words = &wordcache[&IDX_ALL_WORDS];
    let valid_solutions = &wordcache[&IDX_VALID_SOLUTIONS];

    let required_chars: [Charmask; N_SOLUTIONS] = array_init::from_iter(
        valid_solutions.iter().map(|s| s.charmask & guess.charmask)
    ).unwrap();
    let mut banned_chars: [Charmask; WORD_LENGTH*N_SOLUTIONS] = [0; WORD_LENGTH*N_SOLUTIONS];
    /* array_init::from_iter(
        valid_solutions.iter().map(|s| s.charmask & guess.charmask)
    ).unwrap(); */
    for i in 0..N_SOLUTIONS {
        let s = valid_solutions[i];
        let bans = guess.charmask & !s.charmask;  // A letter fully rejected in any position bans it in all positions
        for j in 0..WORD_LENGTH {
            banned_chars[i*WORD_LENGTH + j] = bans;
            banned_chars[i*WORD_LENGTH + j] |= guess.charbits[j] & !s.charbits[j];  // A letter in the wrong position
            // A correct letter bans all others in the position. TODO: test branchless toggle
            let correct = guess.charbits[j] & s.charbits[j];
            //Branch
            /* if correct != 0 {
                banned_chars[i*WORD_LENGTH + j] |= !correct;    
            } */
            //Branchless
            banned_chars[i*WORD_LENGTH + j] |= !correct * (correct !=0) as i32;
        }
    }

    let mut worst = 0;
    let mut worst_w = 0;
    for target_id in 0..N_SOLUTIONS {   
        let cachekey = required_chars[target_id];
        if wordcache.contains_key(&cachekey) {
            let mut remaining = 0;
            for word in &wordcache[&cachekey] {
                // TODO: test branchless toggle
                let mut error = 0;
                for c in 0..WORD_LENGTH {
                    error += word.charbits[c] & banned_chars[target_id*WORD_LENGTH + c];
                }
                remaining += (error == 0) as usize;
            }
            if remaining > worst {
                worst = remaining;
                worst_w = target_id;
            }
        }
    }
    
    let wordstr: String = charbits2str(guess.charbits);  // THIS IS NOT SUITED FOR AGGREGATE GUESSES YET!
    let worststr: String = charbits2str(valid_words[worst_w].charbits);
    let output = format!("{} - {} ({})", wordstr, worst, worststr);
    (output, worst)
}

fn find_word_id_from_str(s: &str, words: &Vec<Word>) -> usize {
    let w = str2word(s);
    words.iter().position(|x| x.charbits==w.charbits).unwrap()
}

fn main() {
    fs::write("test.txt", ["test1", "test2", "test3"].join("\n")).expect("Failed to write output");
    let words = load_dictionary("words-kura");
    let totalwords = words.len();
    println!("Hello, world! {} words in dict", totalwords);
    let wordcache = generate_wordcache(words);
    let all_words = &wordcache[&IDX_ALL_WORDS];

    //let sr = simulate(&wordcache[""][0], &wordcache[""][5000], &wordcache);
    //println!("{:?}", sr);
    
    //(0..=5).flat_map(|i| (i..=5).map(move |j| (i,j))).map(|(i,j)| print!("{},{}\t", i, j));
    
    // Depth-2 full
    // let mut results: Vec<(String, usize)> =
    //    (0..totalwords).into_par_iter().flat_map_iter(|i| (i..totalwords).map(move |j| (i,j)))
    //    .map(|(i, j)| find_worstcase([i, j], &wordcache)).collect();
    
    // Depth-1 full
    let mut results: Vec<(String, usize)> = (0..totalwords).into_par_iter().map(|i| simulate(all_words[i], &wordcache)).collect();
    for _ in 0..9 {  // Benching
        results = (0..totalwords).into_par_iter().map(|i| simulate(all_words[i], &wordcache)).collect();
    }

    // Depth-3 (word1,word2,?)
    // let i1 = find_word_id_from_str("CARET", &wordcache[&0]);
    // let i2 = find_word_id_from_str("SOLID", &wordcache[&0]);
    // let i3 = find_word_id_from_str("NYMPH", &wordcache[&0]);
    // let i4 = find_word_id_from_str("FIFTH", &wordcache[&0]);
    // let mut results: Vec<(String, usize)> =
    //    (0..totalwords).into_par_iter().map(|i| find_worstcase([i1, i2, i3, i4, i], &wordcache)).collect();
    
    results.sort_by_key(|r| r.1);
    let results_strs: Vec<String> = results.iter().map(|r| r.0.clone()).collect();
    fs::write("results.txt", results_strs.join("\n")).expect("Failed to write output");

    // let mut cachekeys: Vec<String> = wordcache.keys().map(|k| charmask2str(*k)).collect();
    // cachekeys.sort();
    // println!("{:?}", cachekeys);
}
