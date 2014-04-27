#![feature(phase)]

extern crate regex;
#[phase(syntax)]
extern crate regex_macros;

fn main() {
    let _ =
        {
            fn exec<'t>(which: ::regex::native::MatchKind, input: &'t str,
                        start: uint, end: uint) -> Vec<Option<uint>> {
                #![allow(unused_imports)]
                use regex::native::{MatchKind, Exists, Location, Submatches,
                                    StepState, StepMatchEarlyReturn,
                                    StepMatch, StepContinue, CharReader,
                                    find_prefix};
                return Nfa{which: which,
                           input: input,
                           ic: 0,
                           chars: CharReader::new(input),}.run(start, end);
                type Captures = [Option<uint>, ..2u];
                struct Nfa<'t> {
                    which: MatchKind,
                    input: &'t str,
                    ic: uint,
                    chars: CharReader<'t>,
                }
                impl <'t> Nfa<'t> {
                    #[allow(unused_variable)]
                    fn run(&mut self, start: uint, end: uint) ->
                     Vec<Option<uint>> {
                        let mut matched = false;
                        let prefix_bytes: &[u8] =
                            &[104u8, 105u8, 112u8, 112u8, 111u8];
                        let mut clist = &mut Threads::new(self.which);
                        let mut nlist = &mut Threads::new(self.which);
                        let mut groups = [None, None];
                        self.ic = start;
                        let mut next_ic = self.chars.set(start);
                        while self.ic <= end {
                            if clist.size == 0 {
                                if matched { break  }
                                if clist.size == 0 {
                                    let haystack =
                                        self.input.as_bytes().slice_from(self.ic);
                                    match find_prefix(prefix_bytes, haystack)
                                        {
                                        None => break ,
                                        Some(i) => {
                                            self.ic += i;
                                            next_ic = self.chars.set(self.ic);
                                        }
                                    }
                                }
                            }
                            if clist.size == 0 || (!false && !matched) {
                                self.add(clist, 0, &mut groups)
                            }
                            self.ic = next_ic;
                            next_ic = self.chars.advance();
                            let mut i = 0;
                            while i < clist.size {
                                let pc = clist.pc(i);
                                let step_state =
                                    self.step(&mut groups, nlist,
                                              clist.groups(i), pc);
                                match step_state {
                                    StepMatchEarlyReturn =>
                                    return {
                                               let mut _temp =
                                                   ::std::vec::Vec::new();
                                               _temp.push(Some(0u));
                                               _temp.push(Some(0u));
                                               _temp
                                           },
                                    StepMatch => {
                                        matched = true;
                                        clist.empty()
                                    },
                                    StepContinue => { }
                                }
                                i += 1;
                            }
                            ::std::mem::swap(&mut clist, &mut nlist);
                            nlist.empty();
                        }
                        match self.which {
                            Exists if matched => {
                                let mut _temp = ::std::vec::Vec::new();
                                _temp.push(Some(0u));
                                _temp.push(Some(0u));
                                _temp
                            },
                            Exists => {
                                let mut _temp = ::std::vec::Vec::new();
                                _temp.push(None);
                                _temp.push(None);
                                _temp
                            },
                            Location | Submatches =>
                            groups.iter().map(|x| *x).collect()
                        }
                    }
                    #[allow(unused_variable)]
                    #[inline]
                    fn step(&self, groups: &mut Captures, nlist: &mut Threads,
                            caps: &mut Captures, pc: uint) -> StepState {
                        match pc {
                            0u => { },
                            1u => {
                                if self.chars.prev == Some('h') {
                                    self.add(nlist, 2u, caps);
                                }
                            },
                            2u => {
                                if self.chars.prev == Some('i') {
                                    self.add(nlist, 3u, caps);
                                }
                            },
                            3u => {
                                if self.chars.prev == Some('p') {
                                    self.add(nlist, 4u, caps);
                                }
                            },
                            4u => {
                                if self.chars.prev == Some('p') {
                                    self.add(nlist, 5u, caps);
                                }
                            },
                            5u => {
                                if self.chars.prev == Some('o') {
                                    self.add(nlist, 6u, caps);
                                }
                            },
                            6u => {
                                if self.chars.prev.is_some() {
                                    let c = self.chars.prev.unwrap();
                                    let found =
                                        match c {
                                            '0' ..'9' => true,
                                            'a' ..'f' => true,
                                            'x' ..'z' => true,
                                            _ => false
                                        };
                                    if found { self.add(nlist, 7u, caps); }
                                }
                            },
                            7u => { },
                            8u => { },
                            9u => {
                                match self.which {
                                    Exists => { return StepMatchEarlyReturn },
                                    Location => {
                                        groups[0] = caps[0];
                                        groups[1] = caps[1];
                                        return StepMatch
                                    },
                                    Submatches => {
                                        match &mut groups.mut_iter().zip(caps.iter())
                                            {
                                            i =>
                                            loop  {
                                                match i.next() {
                                                    None => break ,
                                                    Some((slot, val)) => {
                                                        *slot = *val;
                                                    }
                                                }
                                            }
                                        }
                                        return StepMatch
                                    }
                                }
                            },
                            _ => { }
                        }
                        StepContinue
                    }
                    fn add(&self, nlist: &mut Threads, pc: uint,
                           groups: &mut Captures) {
                        if nlist.contains(pc) { return }
                        match pc {
                            0u => {
                                nlist.add_empty(0u);
                                match self.which {
                                    Submatches | Location => {
                                        let old = groups[0u];
                                        groups[0u] = Some(self.ic);
                                        self.add(nlist, 1u, &mut *groups);
                                        groups[0u] = old;
                                    },
                                    Exists => {
                                        self.add(nlist, 1u, &mut *groups);
                                    }
                                }
                            },
                            1u => nlist.add(1u, &*groups),
                            2u => nlist.add(2u, &*groups),
                            3u => nlist.add(3u, &*groups),
                            4u => nlist.add(4u, &*groups),
                            5u => nlist.add(5u, &*groups),
                            6u => nlist.add(6u, &*groups),
                            7u => {
                                nlist.add_empty(7u);
                                self.add(nlist, 6u, &mut *groups);
                                self.add(nlist, 8u, &mut *groups);
                            },
                            8u => {
                                nlist.add_empty(8u);
                                match self.which {
                                    Submatches | Location => {
                                        let old = groups[1u];
                                        groups[1u] = Some(self.ic);
                                        self.add(nlist, 9u, &mut *groups);
                                        groups[1u] = old;
                                    },
                                    Exists => {
                                        self.add(nlist, 9u, &mut *groups);
                                    }
                                }
                            },
                            9u => nlist.add(9u, &*groups),
                            _ => { }
                        }
                    }
                }
                struct Thread {
                    pc: uint,
                    groups: Captures,
                }
                struct Threads {
                    which: MatchKind,
                    queue: [Thread, ..10u],
                    sparse: [uint, ..10u],
                    size: uint,
                }
                impl Threads {
                    fn new(which: MatchKind) -> Threads {
                        Threads{which: which,
                                queue: unsafe { ::std::mem::uninit() },
                                sparse: unsafe { ::std::mem::uninit() },
                                size: 0,}
                    }
                    #[inline]
                    fn add(&mut self, pc: uint, groups: &Captures) {
                        let t = &mut self.queue[self.size];
                        t.pc = pc;
                        match self.which {
                            Exists => { },
                            Location => {
                                t.groups[0] = groups[0];
                                t.groups[1] = groups[1];
                            },
                            Submatches => {
                                match &mut t.groups.mut_iter().zip(groups.iter())
                                    {
                                    i =>
                                    loop  {
                                        match i.next() {
                                            None => break ,
                                            Some((slot, val)) => {
                                                *slot = *val;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        self.sparse[pc] = self.size;
                        self.size += 1;
                    }
                    #[inline]
                    fn add_empty(&mut self, pc: uint) {
                        self.queue[self.size].pc = pc;
                        self.sparse[pc] = self.size;
                        self.size += 1;
                    }
                    #[inline]
                    fn contains(&self, pc: uint) -> bool {
                        let s = self.sparse[pc];
                        s < self.size && self.queue[s].pc == pc
                    }
                    #[inline]
                    fn empty(&mut self) { self.size = 0; }
                    #[inline]
                    fn pc(&self, i: uint) -> uint { self.queue[i].pc }
                    #[inline]
                    fn groups<'r>(&'r mut self, i: uint) -> &'r mut Captures {
                        &mut self.queue[i].groups
                    }
                }
            }
            ::regex::Regex{original: ~"hippo[a-fx-z0-9]+",
                           names: ~[],
                           p: ::regex::native::Native(exec),}
        };
}
