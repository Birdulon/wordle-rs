use std::fs;
use std::collections::HashMap;
use bitintr::{Lzcnt, Tzcnt};
use regex::Regex;
use rayon::prelude::*;
use itertools::zip;

type Charmask = i32;
type Achar = i8;  // ASCII char

const WORD_LENGTH: usize = 5;
const WORD_LENGTH_P: usize = 5;  // Padded for SIMD shenanigans
const GUESS_DEPTH: usize = 2;
const A: Achar = 'A' as Achar;
const Z: Achar = 'Z' as Achar;

#[derive(Copy, Clone, Default)]
struct SimState {
    banned_chars: [Charmask; WORD_LENGTH_P],  // Alphabetical bitmask
    required_chars: Charmask
}

#[derive(Copy, Clone, Default)]
struct Word {
    charbits: [Charmask; WORD_LENGTH_P],  // Each letter in bitmask form
    charmask: Charmask,                   // All of the characters contained
    letters: [Achar; WORD_LENGTH]
}

type WordCache = HashMap<Charmask, Vec<Word>>;

fn str2word(s: &str) -> Word {
    let mut word = Word::default();
    let mut iter = s.chars();
    for i in 0..WORD_LENGTH {
        let c = iter.next().unwrap() as Achar;
        let cb = char2bit(c);
        word.charbits[i] = cb;
        word.letters[i] = c;
        word.charmask |= cb;
    }
    word
}

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
    words.sort();
    words.dedup();
    words.iter().map(|w| str2word(w)).collect()
}

fn letters2str(letters: [Achar; WORD_LENGTH]) -> String {
    letters.iter().map(|x| (*x as u8) as char).collect()
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


fn char2bit(c: Achar) -> Charmask {
    debug_assert!((A..=Z).contains(&c));
    1 << (c - A)
}

fn cm2char(cm: Charmask, offset: i8) -> Achar {
    (((31 - cm.lzcnt() as i8) + A + offset) as u8) as Achar
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

fn generate_wordcache(words: Vec<Word>) -> WordCache {
    let mut cache: WordCache = HashMap::new();
    let subcache: Vec<Word> = words.to_vec();
    _generate_wordcache_nested(&mut cache, &subcache, 0, 5);
    cache.insert(0, words);
    cache
}

fn filter_word(w: &[Charmask; WORD_LENGTH_P], banned_chars: &[Charmask; WORD_LENGTH_P]) -> bool {
    zip(w, banned_chars).all(|(x,y)| x & y == 0)
}

fn simulate(guess_ids: [usize; GUESS_DEPTH], solution_id: usize, mut s: SimState, wordcache: &WordCache) -> usize {
    let allwords = &wordcache[&0];
    let solution = allwords[solution_id];
    let mut bans = 0;
    for guess_id in guess_ids {
        let guess = allwords[guess_id];
        s.required_chars |= guess.charmask & solution.charmask;
        bans |= guess.charmask & !solution.charmask;
        for i in 0..WORD_LENGTH {
            if guess.letters[i] == solution.letters[i] {  // Right letter right position
                s.banned_chars[i] = !guess.charbits[i];
            } else if guess.charbits[i] & solution.charmask != 0 {  // Right letter wrong position
                s.banned_chars[i] |= guess.charbits[i];
            }
        }
    }
    for j in 0..s.banned_chars.len() {
        s.banned_chars[j] |= bans;
    }
    let cachekey = s.required_chars;
    match wordcache.contains_key(&cachekey) {
        true => wordcache[&cachekey].iter().filter(|w| filter_word(&w.charbits, &s.banned_chars)).count(),
        false => 0,
    }
}

fn find_worstcase(word_ids: [usize; GUESS_DEPTH], wordcache: &WordCache) -> (String, usize) {
    let allwords = &wordcache[&0];

    let mut worst = 0;
    let mut worst_w = 0;
    let ss = SimState::default();
    for target_id in 0..allwords.len() {
        let remaining = simulate(word_ids, target_id, ss, wordcache);
        if remaining > worst {
            worst = remaining;
            worst_w = target_id;
        };
    }
    let wordstr: String = word_ids.map(|i| letters2str(allwords[i].letters)).join(", ");
    let worststr: String = letters2str(allwords[worst_w].letters);
    let output = format!("{} - {} ({})", wordstr, worst, worststr);
    println!("{}", output);
    (output, worst)
}

fn main() {
    fs::write("test.txt", ["test1", "test2", "test3"].join("\n")).expect("Failed to write output");
    let words = load_dictionary("words");
    let totalwords = words.len();
    println!("Hello, world! {} words in dict", totalwords);
    let wordcache = generate_wordcache(words);

    //let sr = simulate(&wordcache[""][0], &wordcache[""][5000], &wordcache);
    //println!("{:?}", sr);
    
    //(0..=5).flat_map(|i| (i..=5).map(move |j| (i,j))).map(|(i,j)| print!("{},{}\t", i, j));
    let mut results: Vec<(String, usize)> =
       (0..totalwords).into_par_iter().flat_map_iter(|i| (i..totalwords).map(move |j| (i,j)))
       .map(|(i, j)| find_worstcase([i, j], &wordcache)).collect();
    // let mut results: Vec<(String, usize)> =
    //    (0..totalwords).into_par_iter()
    //    .map(|i| find_worstcase([i], &wordcache)).collect();
    results.sort_by_key(|r| r.1);
    let results_strs: Vec<String> = results.iter().map(|r| r.0.clone()).collect();
    fs::write("results.txt", results_strs.join("\n")).expect("Failed to write output");

    // let mut cachekeys: Vec<String> = wordcache.keys().map(|k| charmask2str(*k)).collect();
    // cachekeys.sort();
    // println!("{:?}", cachekeys);
}
