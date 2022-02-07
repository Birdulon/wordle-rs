#![allow(dead_code)]
#![allow(unused_imports)]
use std::{env, fs};
// use bitintr::Lzcnt;
use regex::Regex;
use rayon::prelude::*;
use itertools::zip;
use array_init::array_init;
use std::collections::BTreeMap;

pub type Charmask = i128;

pub const WORD_LENGTH: usize = 4;
pub const WORD_LENGTH_P: usize = 4;  // Padded for SIMD shenanigans
pub const GUESS_DEPTH: usize = 1;  // TODO: Change this whenever working at different depths
pub const N_LETTERS: u8 = 74;
// pub const n_solutions: usize = 2315;
pub const CACHE_SIZE: usize = 1<<26;
pub const IDX_ALL_WORDS: Charmask = (CACHE_SIZE as Charmask) - 1;
pub const IDX_VALID_SOLUTIONS: Charmask = 0;

pub const MAX_ENTRIES_PER_JOB: usize = 1000;

#[derive(Copy, Clone, Default)]
pub struct Word {
    charbits: [Charmask; WORD_LENGTH_P],  // Each letter in bitmask form
    charmask: Charmask,                   // All of the characters contained
}


#[cfg(use_thin_array)]
mod thin_array;
#[cfg(use_thin_array)]
type WordCache = thin_array::ThinArray<Vec<Word>, CACHE_SIZE, 7000>;

#[cfg(all(not(use_thin_array), use_hashmap))]
use std::collections::HashMap;
#[cfg(all(not(use_thin_array), use_hashmap))]
type WordCache = HashMap<Charmask, Vec<Word>>;  // Default hash is slower than BTree on M1

#[cfg(all(not(use_thin_array), not(use_hashmap), feature = "ahash"))]
use std::collections::HashMap;
#[cfg(all(not(use_thin_array), not(use_hashmap), feature = "ahash"))]
use ahash::{AHasher, RandomState};
#[cfg(all(not(use_thin_array), not(use_hashmap), feature = "ahash"))]
type WordCache = HashMap<Charmask, Vec<Word>, RandomState>;

#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), feature = "xxhash_rust"))]
use std::collections::HashMap;
#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), feature = "xxhash_rust"))]
use xxhash_rust::xxh3::Xxh3;
#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), feature = "xxhash_rust"))]
use std::hash::BuildHasherDefault;
#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), feature = "xxhash_rust"))]
type WordCache = HashMap<Charmask, Vec<Word>, BuildHasherDefault<Xxh3>>;

#[cfg(all(not(use_thin_array), not(use_hashmap), not(feature = "ahash"), not(feature = "xxhash_rust")))]
type WordCache = BTreeMap<Charmask, Vec<Word>>;


fn default_wordcache() -> WordCache {
    WordCache::default()
}

fn char2bit(c: char) -> Charmask {
    match c {  // By setting the most frequently-occurring kana to the highest bits, we can numerically assess what word combinations have more of them set
        'ん' => 1<<73,
        'い' => 1<<72,
        'う' => 1<<71,
        'か' => 1<<70,
        'る' => 1<<69,
        'く' => 1<<68,
        'つ' => 1<<67,
        'こ' => 1<<66,
        'し' => 1<<65,
        'と' => 1<<64,
        'た' => 1<<63,
        'き' => 1<<62,
        'す' => 1<<61,
        'せ' => 1<<60,
        'さ' => 1<<59,
        'お' => 1<<58,
        'ま' => 1<<57,
        'な' => 1<<56,
        'け' => 1<<55,
        'ら' => 1<<54,
        'て' => 1<<53,
        'れ' => 1<<52,
        'り' => 1<<51,
        'あ' => 1<<50,
        'が' => 1<<49,
        'だ' => 1<<48,
        'ち' => 1<<47,
        'そ' => 1<<46,
        'め' => 1<<45,
        'え' => 1<<44,
        'ど' => 1<<43,
        'は' => 1<<42,
        'じ' => 1<<41,
        'も' => 1<<40,
        'よ' => 1<<39,
        'ー' => 1<<38,
        'ろ' => 1<<37,
        'の' => 1<<36,
        'ぶ' => 1<<35,
        'げ' => 1<<34,
        'み' => 1<<33,
        'や' => 1<<32,
        'わ' => 1<<31,
        'に' => 1<<30,
        'ふ' => 1<<29,
        'ほ' => 1<<28,
        'ば' => 1<<27,
        'ぼ' => 1<<26,
        'ひ' => 1<<25,
        'ざ' => 1<<24,
        'ご' => 1<<23,
        'ず' => 1<<22,
        'ゆ' => 1<<21,
        'ぞ' => 1<<20,
        'む' => 1<<19,
        'び' => 1<<18,
        'で' => 1<<17,
        'ぜ' => 1<<16,
        'ね' => 1<<15,
        'べ' => 1<<14,
        'ぱ' => 1<<13,
        'へ' => 1<<12,
        'ぐ' => 1<<11,
        'ぎ' => 1<<10,
        'づ' => 1<<9,
        'ぷ' => 1<<8,
        'ぽ' => 1<<7,
        'ぴ' => 1<<6,
        'ぬ' => 1<<5,
        'ぺ' => 1<<4,
        'ぢ' => 1<<3,
        'を' => 1<<2,
        'ゔ' => 1<<1,
        '〜' => 1<<0,
        _ => 0
    }
}


fn str2word(s: &str) -> Word {
    let mut word = Word::default();
    let mut iter = s.chars();
    for i in 0..WORD_LENGTH {
        let c = iter.next().unwrap();
        let cb = char2bit(c);
        word.charbits[i] = cb;
        word.charmask |= cb;
    }
    word
}

/* fn cm2char(cm: Charmask, offset: i8) -> Achar {
    (((31 - cm.lzcnt() as i8) + A + offset) as u8) as Achar
}

fn letters2str(letters: [Achar; WORD_LENGTH]) -> String {
    letters.iter().map(|x| (*x as u8) as char).collect()
}

fn charbits2str(charbits: [Charmask; WORD_LENGTH]) -> String {
    charbits.iter().map(|x| (cm2char(*x, 0) as u8) as char).collect()
} */

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

fn load_dictionary(filename: &str) -> (Vec<String>, usize) {
    println!("Loading dictionary at {}", filename);
    let rawfile = fs::read_to_string(filename).unwrap();
    let rawwords = rawfile.split('\n');
    let mut words = Vec::<String>::new();
    let mut n_solutions = 0;
    for line in rawwords {
        if line == "[Ta]" {
            n_solutions = words.len();
        } else if line.chars().count() == 4 {
            words.push(line.to_string());
        }
    }
    (words, n_solutions)
}

fn _generate_wordcache_nested(cache: &mut WordCache, subcache: &[Word], key: Charmask, next_bit: u8, depth: u8) {
    for b in next_bit..N_LETTERS {
        let cb = 1<<b;
        let sc2: Vec<Word> = subcache.iter().filter(|w| (w.charmask & cb) == cb).cloned().collect();
        if !sc2.is_empty() {
            let key2 = key | cb;
            if depth > 0 {
                _generate_wordcache_nested(cache, &sc2, key2, b+1, depth-1);
            }
            cache.insert(key2, sc2);
        }
    }
}

fn generate_wordcache(valid_words: Vec<Word>, n_solutions: usize) -> WordCache {
    let mut cache: WordCache = default_wordcache();
    let valid_solutions: Vec<Word> = valid_words[..n_solutions].to_vec();  // Hacky way to separate the valid solutions from the larger guessing list
    _generate_wordcache_nested(&mut cache, &valid_solutions, 0, 0, 5);
    cache.insert(IDX_VALID_SOLUTIONS, valid_solutions);
    cache.insert(IDX_ALL_WORDS, valid_words);
    cache
}

fn filter_word(w: &[Charmask; WORD_LENGTH_P], banned_chars: &[Charmask; WORD_LENGTH_P]) -> bool {
    zip(w, banned_chars).all(|(x,y)| x & y == 0)
}

fn aggregate_guesses(guess_ids: &[usize], wordcache: &WordCache) -> Word {
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

fn simulate(guess: Word, wordcache: &WordCache) -> (usize, usize) {
    // let valid_words = &wordcache[&IDX_ALL_WORDS];
    let valid_solutions = &wordcache[&IDX_VALID_SOLUTIONS];
    let n_solutions = valid_solutions.len();

    let required_chars: Vec<Charmask> = valid_solutions.iter().map(|s| s.charmask & guess.charmask).collect();
    let mut banned_chars: Vec<Charmask> = (0..WORD_LENGTH*n_solutions).map(|_| 0).collect();

    for i in 0..n_solutions {
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
            banned_chars[i*WORD_LENGTH + j] |= !correct * (correct !=0) as Charmask;
        }
    }

    let mut worst = 0;
    let mut worst_w = 0;
    for target_id in 0..n_solutions {   
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
    (worst, worst_w)
}

fn calculate_best(w1start: usize, w1end: usize, total: usize, wordcache: &WordCache) -> Vec<(Vec<usize>, (usize, usize))> {
    println!("Starting from word #{} to ending word #{}.", w1start, w1end);
    let mut guess_ids: Vec<Vec<usize>> = Vec::default();
    for i1 in w1start..w1end {
        guess_ids.push(vec![i1])
        // for i2 in i1..total {
        //     guess_ids.push(vec![i1,i2])
        // }
    }
    let guesses: Vec<Word> = guess_ids.iter().map(|i| aggregate_guesses(&i, &wordcache)).collect();
    println!("This consists of {} guess combinations", guess_ids.len());

    let mut results: Vec<(Vec<usize>, (usize, usize))> =
        (0..guess_ids.len()).into_par_iter()
        .map(|i| (guess_ids[i].clone(), simulate(guesses[i], &wordcache)))
        .collect();
    // results.sort_by_key(|(_guess, (worst, _solution))| worst);
    results.sort_by_key(|x| x.1.0);
    println!("Processed {} guesses from starting word #{} to ending word #{}.", results.len(), w1start, w1end);
    results
}

fn guess2str(guess: &[usize], word_strs: &[String]) -> String {
    let strs: Vec<String> = guess.iter().map(|i| word_strs[*i].clone()).collect();
    strs.join(",")
}

fn main() {
    //eprint!("Hello, world!\n");
    // Prints each argument on a separate line
    for argument in env::args() {
        print!("{}\t", argument);
    }
    //fs::write("test.txt", ["test1", "test2", "test3"].join("\n")).expect("Failed to write output");
    let (word_strs, n_solutions) = load_dictionary("kotobade-asobou-list");
    let totalwords = word_strs.len();
    let words: Vec<Word> = word_strs.iter().map(|w| str2word(w)).collect();
    println!("Loaded dict - {} words in dict, {} of which can be solutions.", totalwords, n_solutions);
    let wordcache = generate_wordcache(words, n_solutions);
    //let all_words = &wordcache[&IDX_ALL_WORDS];
    // println!("Cache contains {} keys", wordcache.keys().len());  // 6756 on words-kura

    let args: Vec<String> = env::args().collect();
    let w1start: usize;
    let w1end: usize;
    match args.len() {
        3 => {
            let s_w1start = &args[1];
            let s_w1end = &args[2];
            // parse the numbers
            w1start = match s_w1start.parse() {
                Ok(n) => n,
                Err(_) => {
                    eprintln!("error: not a valid start point");
                    return;
                },
            };
            w1end = match s_w1end.parse() {
                Ok(n) => totalwords.min(n),
                Err(_) => {
                    eprintln!("error: not a valid end point");
                    return;
                },
            };
        },
        _ => {
            w1start = 0;
            w1end = totalwords;
        }
    }
    
    // Depth-1 full
    //let mut results: Vec<(String, usize)> = (0..totalwords).into_par_iter().map(|i| simulate(all_words[i], &wordcache)).collect();
    // for _ in 0..9 {  // Benching
    //     results = (0..totalwords).into_par_iter().map(|i| simulate(all_words[i], &wordcache)).collect();
    // }
    

    let results = calculate_best(w1start, w1end, totalwords, &wordcache);
    let trim = (results.len() - MAX_ENTRIES_PER_JOB).max(0);
    println!("\tBest score {}, worst {}. Discarding worst {} entries to leave maximum of {}.", results[0].1.0, results.last().unwrap().1.0, trim, MAX_ENTRIES_PER_JOB);
    let results_strs: Vec<String> = 
        results.iter().take(MAX_ENTRIES_PER_JOB.min(results.len()))
        .map(
            |(guess, (worst, solution))|  format!("{}\t{} ({})", worst, guess2str(guess, &word_strs), word_strs[*solution])
        ).collect();
    

    //let results_strs: Vec<String> = results.iter().map(|r| r.0.clone()).collect();
    fs::write(format!("results_from_{}_to_{}.txt", w1start, w1end), results_strs.join("\n")).expect("Failed to write output");
}
