use crate::net_impl::SocketSet;
use alloc::collections::VecDeque;
use alloc::vec;
use alloc::vec::Vec;
//use smoltcp::iface::SocketSet;
use smoltcp::phy::DeviceCapabilities;
use smoltcp::phy::Medium;
use smoltcp::phy::{Device, RxToken, TxToken};
use smoltcp::time::Instant;
pub(crate) struct LBDEV {
    pub(crate) queue: VecDeque<Vec<u8>>,
    medium: Medium,
}

impl LBDEV {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            medium: Medium::Ip,
        }
    }
}

pub(crate) struct LBTxToken<'a> {
    queue: &'a mut VecDeque<Vec<u8>>,
}
pub(crate) struct LBRxToken {
    buffer: Vec<u8>,
}
impl Device for LBDEV {
    type RxToken<'a>
        = LBRxToken
    where
        Self: 'a;
    type TxToken<'a>
        = LBTxToken<'a>
    where
        Self: 'a;
    //通信令牌？

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        if let Some(buffer) = self.queue.pop_front() {
            Some((
                LBRxToken { buffer },
                LBTxToken {
                    queue: &mut self.queue,
                },
            ))
        } else {
            None
        }
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(LBTxToken {
            queue: &mut self.queue,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 65535;
        //caps.max_burst_size = None;
        caps.medium = self.medium;
        caps
    }
}
impl RxToken for LBRxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = self.buffer;
        let result = f(&mut buffer);
        result
    }
    fn preprocess(&self, sockets: &mut SocketSet<'_>) {
        snoop_tcp_packet(&self.buffer, sockets).ok();
    }
}

impl<'a> TxToken for LBTxToken<'a> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = vec![0u8; len];
        let result = f(&mut buffer);
        self.queue.push_back(buffer);
        result
    }
}

use crate::net_impl::LISTEN_TABLE;
fn snoop_tcp_packet(buf: &[u8], sockets: &mut SocketSet<'_>) -> Result<(), smoltcp::wire::Error> {
    use smoltcp::wire::{EthernetFrame, IpProtocol, Ipv4Packet, TcpPacket};

    //let ether_frame = EthernetFrame::new_checked(buf)?;
    let ipv4_packet = Ipv4Packet::new_checked(buf)?;

    if ipv4_packet.next_header() == IpProtocol::Tcp {
        let tcp_packet = TcpPacket::new_checked(ipv4_packet.payload())?;
        let src_addr = (ipv4_packet.src_addr(), tcp_packet.src_port()).into();
        let dst_addr = (ipv4_packet.dst_addr(), tcp_packet.dst_port()).into();
        let is_first = tcp_packet.syn() && !tcp_packet.ack();
        if is_first {
            // create a socket for the first incoming TCP packet, as the later accept() returns.
            LISTEN_TABLE.incoming_tcp_packet(src_addr, dst_addr, sockets);
        }
    }
    Ok(())
}
