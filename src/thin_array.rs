use core::ops::Index;

type TKeys = crate::Charmask; // I give up on making this generic for now
pub struct ThinArray<T, const N_KEYS: usize, const CAPACITY: usize> {
    // keys: [TKeys; N_KEYS],
    keys: Vec<TKeys>,
    items: [T; CAPACITY],
    items_used: usize,
}

impl<T: Default, const N_KEYS: usize, const CAPACITY: usize> ThinArray<T, N_KEYS, CAPACITY> {
    pub fn default() -> Self {
        // println!("Initializing ThinArray");
        Self{
            // keys: [0; N_KEYS],
            items: array_init::array_init(|_| T::default()),
            keys: (0..N_KEYS).map(|_| 0).collect(),
            items_used: 0,
        }
    }

    pub fn insert(&mut self, key: TKeys, value: T) {
        // println!("Insert requested for key {}", key);
        debug_assert!(self.items_used < CAPACITY);
        self.items_used += 1;
        self.items[self.items_used as usize] = value;
        // self.items.push(value);
        self.keys[key as usize] = self.items_used as TKeys;
    }

    pub fn get(&self, key: TKeys) -> &T {
        &self.items[self.keys[key as usize] as usize]
    }

    pub fn contains_key(&self, _key: &TKeys) -> bool {
        true
        // key < N_KEYS
    }
}
impl<T: Default, const N_KEYS: usize, const CAPACITY: usize> Index<&TKeys> for ThinArray<T, N_KEYS, CAPACITY> {
    type Output = T;

    fn index(&self, key: &TKeys) -> &T {
        // println!("Key requested: {}", key);
        &self.items[self.keys[*key as usize] as usize]
    }
}