// use crate::{
//     final_ver::{
//         arena_slot_allocator::{ArenaChunkIndex, ArenaIndex, ArenaSlotAllocator},
//         arena_slot_map::ArenaSlot,
//         orderbook::LimitOrder,
//     },
//     slot_map::NonMaxU32,
// };
//
// pub struct ArenaSlotMap {
//     pub head: NonMaxU32,
//     pub tail: NonMaxU32,
//     pub free_head: NonMaxU32,
//     pub owned_chunks: Vec<ArenaChunkIndex>,
//     pub capacity: usize,
//     pub total_len: usize,
// }
//
// impl ArenaSlotMap {
//     pub fn from_arena(arena: &mut ArenaSlotAllocator) -> Self {
//         let chunk_size = arena.chunk_size();
//         let chunk_index = unsafe { arena.claim_chunk() };
//
//         let mut this = Self {
//             head: NonMaxU32::new_none(),
//             tail: NonMaxU32::new_none(),
//             free_head: NonMaxU32::new_none(),
//             // Pre-allocate vector capacity to avoid heap jitter during runtime push()
//             owned_chunks: Vec::with_capacity(16),
//             capacity: 0,
//             total_len: 0,
//         };
//
//         // Initialize the very first chunk's slots into the free list
//         unsafe {
//             this.link_new_chunk(&chunk_index, arena);
//         }
//
//         this.owned_chunks.push(chunk_index);
//         this.capacity += chunk_size;
//         this
//     }
//
//     /// Links all slots in a newly claimed chunk into the `free_head` chain.
//     unsafe fn link_new_chunk(
//         &mut self,
//         chunk_index: &ArenaChunkIndex,
//         arena: &mut ArenaSlotAllocator,
//     ) {
//         let chunk_size = arena.chunk_size();
//         let start_idx = chunk_index.0 * chunk_size;
//         let end_idx = start_idx + chunk_size;
//
//         // Chain the new slots together from back to front
//         for i in (start_idx..end_idx).rev() {
//             let slot_ref = arena.get_unchecked_mut(i);
//
//             // Setting up initial generation to 0, pointing to the previous free_head
//             *slot_ref = ArenaSlot::Free {
//                 generation: 0,
//                 next_free: self.free_head,
//             };
//             self.free_head = NonMaxU32::from(i as u32);
//         }
//     }
//
//     #[allow(clippy::cast_possible_truncation)]
//     pub fn insert(&mut self, data: LimitOrder, arena: &mut ArenaSlotAllocator) -> ArenaIndex {
//         // Cold path: If we are entirely out of free slots, dynamically pull a new chunk
//         if self.free_head.is_none() {
//             unsafe {
//                 let new_chunk_index = arena.claim_chunk();
//                 self.owned_chunks.push(new_chunk_index);
//                 self.capacity += arena.chunk_size();
//                 self.link_new_chunk(&new_chunk_index, arena);
//             }
//         }
//
//         // Grab the next available slot from the free list
//         let free_index = self.free_head.0;
//         let slot_generation;
//
//         unsafe {
//             let free_slot_ref = arena.get_unchecked(free_index as usize);
//             match free_slot_ref {
//                 ArenaSlot::Free {
//                     generation,
//                     next_free,
//                 } => {
//                     slot_generation = *generation;
//                     self.free_head = *next_free;
//                 }
//                 _ => std::hint::unreachable_unchecked(),
//             }
//         }
//
//         // Update the current tail to point forward to our new node
//         let tail_index = self.tail;
//         if tail_index.is_some() {
//             unsafe {
//                 let (_, _, _, next) = arena
//                     .get_unchecked_mut(tail_index.0 as usize)
//                     .as_occupied_unchecked_mut();
//                 next.0 = free_index;
//             }
//         }
//
//         // Overwrite the free slot with our Occupied data
//         let new_occupied = ArenaSlot::Occupied {
//             generation: slot_generation,
//             data,
//             prev: tail_index,
//             next: NonMaxU32::new_none(),
//         };
//
//         unsafe {
//             let free_slot_ref = arena.get_unchecked_mut(free_index as usize);
//             *free_slot_ref = new_occupied;
//         };
//
//         if self.head.is_none() {
//             self.head = NonMaxU32::from(free_index);
//         }
//
//         self.tail = NonMaxU32::from(free_index);
//         self.total_len += 1;
//
//         // Return the combined index and generation tag
//         ArenaIndex {
//             index: free_index,
//             generation: slot_generation,
//         }
//     }
//
//     pub fn remove(&mut self, remove_key: &ArenaIndex, arena: &mut ArenaSlotAllocator) -> bool {
//         let index = remove_key.index as usize;
//
//         // 1. SAFE CHECK: Ensure we aren't indexing completely out of bounds of allocated capacity
//         if index >= self.capacity {
//             return false;
//         }
//
//         let slot_ref = unsafe { arena.get_unchecked(index) };
//
//         // 2. GENERATIONAL CHECK: Avoid double-frees and dangling key access
//         match slot_ref {
//             ArenaSlot::Occupied {
//                 generation,
//                 prev,
//                 next,
//                 ..
//             } => {
//                 if *generation != remove_key.generation {
//                     return false; // Outdated generation key, ignore safely
//                 }
//
//                 let curr_prev = *prev;
//                 let curr_next = *next;
//                 let current_generation = *generation;
//
//                 // Sever relationships in the doubly linked list
//                 if curr_prev.is_some() {
//                     unsafe {
//                         let (_, _, _, next) = arena
//                             .get_unchecked_mut(curr_prev.0 as usize)
//                             .as_occupied_unchecked_mut();
//                         *next = curr_next;
//                     }
//                 }
//
//                 if curr_next.is_some() {
//                     unsafe {
//                         let (_, _, prev, _) = arena
//                             .get_unchecked_mut(curr_next.0 as usize)
//                             .as_occupied_unchecked_mut();
//                         *prev = curr_prev;
//                     }
//                 }
//
//                 // Turn the slot back into a Free slot, INCREMENTING the generation
//                 unsafe {
//                     let remove_slot_ref = arena.get_unchecked_mut(index);
//                     *remove_slot_ref = ArenaSlot::Free {
//                         generation: current_generation.wrapping_add(1),
//                         next_free: self.free_head,
//                     };
//                 }
//
//                 self.free_head = NonMaxU32::from(remove_key.index);
//
//                 if curr_next.is_none() {
//                     self.tail = curr_prev;
//                 }
//
//                 if curr_prev.is_none() {
//                     self.head = curr_next;
//                 }
//
//                 // Explicitly allow debug underflow panics to catch architecture bugs early
//                 self.total_len -= 1;
//
//                 self.total_len == 0
//             }
//             ArenaSlot::Free { .. } => false, // Already freed! Safe early return.
//         }
//     }
// }
