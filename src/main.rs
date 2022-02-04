use std::fs;
use std::collections::HashMap;
use bitintr::{Lzcnt, Tzcnt};
use regex::Regex;
use rayon::prelude::*;

type Charmask = i32;
type Achar = i8;  // ASCII char

const WORD_LENGTH: usize = 5;
const WORD_LENGTH_P: usize = 5;  // Padded for SIMD shenanigans
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


fn char2bit(c: Achar) -> Charmask {
    debug_assert!((A..=Z).contains(&c));
    1 << (c - A)
}

fn cm2char(cm: Charmask, offset: i8) -> Achar {
    (((31 - cm.lzcnt() as i8) + A + offset) as u8) as Achar
    //(((cm.tzcnt() as i8) + A + offset) as u8) as Achar
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

fn filter_word(w: &Word, banned_chars: &[Charmask; WORD_LENGTH_P]) -> bool {
    for (cb, bans) in w.charbits.iter().zip(banned_chars.iter()) {
        if cb & bans != 0 {
            return false;
        }
    }
    true
}

fn simulate(guess: &Word, solution: &Word, mut s: SimState, wordcache: &WordCache) -> (usize, SimState) {
    s.required_chars |= guess.charmask & solution.charmask;
    for (i, (gc, sc)) in guess.letters.iter().zip(solution.letters.iter()).enumerate() {
        let gb = char2bit(*gc);
        if gc == sc {  // Right letter right position
            s.banned_chars[i] = 255 ^ gb;
        } else if solution.charmask & gb != 0 {  // Right letter wrong position
            s.banned_chars[i] |= gb;
        } else {  // Letter not in solution
            for j in 0..s.banned_chars.len() {
                s.banned_chars[j] |= gb;
            }
        }
    }
    let cachekey = s.required_chars;
    match wordcache.contains_key(&cachekey) {
        true => (
            wordcache[&cachekey].iter().filter(|w| filter_word(w, &s.banned_chars)).count(),
            s
        ),
        false => (
            0,
            s
        ),
    }
}

fn find_worstcase(word: &Word, wordcache: &WordCache) -> (String, usize) {
    let mut worst = 0;
    let mut worst_w = wordcache[&0][0].letters;
    let ss = SimState::default();
    for target in &wordcache[&0] {
        let remaining = simulate(word, target, ss, &wordcache).0;
        if remaining > worst {
            worst = remaining;
            worst_w = target.letters;
        };
    }
    let wordstr: String = word.letters.iter().map(|x| (*x as u8) as char).collect();
    let worststr: String = worst_w.iter().map(|x| (*x as u8) as char).collect();
    let output = format!("{} - {} ({})", wordstr, worst, worststr);
    println!("{}", output);
    (output, worst)
}

fn charmask2str(cm: Charmask) -> String {
    let mut s = String::default();
    for i in cm.tzcnt() ..= 32-cm.lzcnt() {
        if (cm & (1<<i)) != 0 {
            s += &((A + i as Achar) as u8 as char).to_string();
        }
    }
    s
}

fn main() {
    fs::write("test.txt", ["test1", "test2", "test3"].join("\n")).expect("Failed to write output");
    let words = load_dictionary("words");
    println!("Hello, world! {} words in dict", words.len());
    let wordcache = generate_wordcache(words);

    //let sr = simulate(&wordcache[""][0], &wordcache[""][5000], &wordcache);
    //println!("{:?}", sr);
    
    let mut results: Vec<(String, usize)> = wordcache[&0].par_iter().map(|w| find_worstcase(w, &wordcache)).collect();
    results.sort_by_key(|r| r.1);
    let results_strs: Vec<String> = results.iter().map(|r| r.0.clone()).collect();
    fs::write("results.txt", results_strs.join("\n")).expect("Failed to write output");

    // let mut cachekeys: Vec<String> = wordcache.keys().map(|k| charmask2str(*k)).collect();
    // cachekeys.sort();
    // println!("{:?}", cachekeys);
}
