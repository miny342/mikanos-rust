use core::task::Poll;

use futures_util::Stream;

use crate::usb::controller::{ER_WAKER, XhcController};
use crate::usb::trb::TRB;


pub const TRB_BUF_LEN: usize = 32;

#[repr(align(64))]
pub struct TRBTable(pub [TRB; TRB_BUF_LEN]);

pub struct MemPoolCrTRB {
    pub x: &'static mut TRBTable, // [TRB; TRB_BUF_LEN],
    pub index: usize,
    pub cycle: bool
}

impl MemPoolCrTRB {
    pub fn push(&mut self, mut trb: TRB) {
        trb.data[3] = (trb.data[3] & !0x1) | (self.cycle as u32);
        for i in 0..4 {
            self.x.0[self.index].data[i] = trb.data[i]
        }
        self.index += 1;
        if self.index == TRB_BUF_LEN - 1 {
            let mut link = TRB::new_link_trb(self.x.0.as_ptr() as u64);
            link.data[3] = link.data[3] | (self.cycle as u32);
            for i in 0..4 {
                self.x.0[self.index].data[i] = link.data[i];
            }
            self.index = 0;
            self.cycle = !self.cycle;
        }
    }
}

#[repr(C, packed)]
pub struct EventRingSegmentTableEntry {
    pub addr: u64,
    pub size: u16,
    rsvdz1: u16,
    rsvdz2: u32,
}

#[repr(align(64))]
pub struct MemPoolERSTE {
    pub(crate) x: [EventRingSegmentTableEntry; 1]
}

pub struct MemPoolErTRB {
    pub x: &'static TRBTable, // [TRB; TRB_BUF_LEN],
    pub index: usize,
    pub cycle: bool,
}

impl MemPoolErTRB {
    pub fn next_(&mut self) -> Option<TRB> {
        let v = self.x.0[self.index];
        if v.cycle() == self.cycle {
            if self.index == TRB_BUF_LEN - 1 {
                self.index = 0;
                self.cycle = !self.cycle;
            } else {
                self.index += 1;
            }
            Some(v)
        } else {
            None
        }
    }
    pub fn clean(&self, xhc: &XhcController) {
        unsafe {
            let interrupt_reg = xhc.capability.runtime().interrupt_set();
            let p = interrupt_reg[0].event_ring_dequeue_pointer.read() & 0xf;
            interrupt_reg[0].event_ring_dequeue_pointer.write(p | (&self.x.0[self.index] as *const TRB as u64));
        }
    }
}

impl Stream for MemPoolErTRB {
    type Item = TRB;

    fn poll_next(self: core::pin::Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> core::task::Poll<Option<Self::Item>> {
        let v = self.get_mut();
        if let Some(trb) = v.next_() {
            return Poll::Ready(Some(trb))
        }

        ER_WAKER.register(&cx.waker());
        match v.next_() {
            Some(trb) => {
                ER_WAKER.take();
                Poll::Ready(Some(trb))
            }
            None => Poll::Pending
        }
    }
}

#[repr(C, align(64))]
pub struct MemPoolTrTRB {
    pub x: &'static mut TRBTable, // [TRB; TRB_BUF_LEN],
    pub index: usize,
    pub cycle: bool,
}

impl MemPoolTrTRB {
    pub fn center(&self) -> &TRB {
        &self.x.0[self.index]
    }
    pub fn push(&mut self, mut trb: TRB) {
        trb.data[3] = (trb.data[3] & !0x1) | (self.cycle as u32);
        for i in 0..4 {
            self.x.0[self.index].data[i] = trb.data[i]
        }
        self.index += 1;
        if self.index == TRB_BUF_LEN - 1 {
            let mut link = TRB::new_link_trb(self.x.0.as_ptr() as u64);
            link.data[3] = (link.data[3] & !0x1) | (self.cycle as u32);
            for i in 0..4 {
                self.x.0[self.index].data[i] = link.data[i];
            }
            self.index = 0;
            self.cycle = !self.cycle;
        }
    }
}
