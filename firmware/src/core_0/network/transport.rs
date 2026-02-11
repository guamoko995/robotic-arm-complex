use crate::{
    connectors::{PosAckReceiver, PosSender},
    core_0::{
        connectors::{CmdAckReceiver, CmdSender},
        mk_static,
        network::{ActiveWifiInterface, api, connectors::WifiInterface},
    },
};
use core::{
    mem,
    net::Ipv4Addr,
    ops::{Deref, DerefMut},
};
use embassy_futures::select::{Either, select};
use embassy_net::{Stack, tcp::TcpSocket};
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, RawMutex};
use embassy_time::{Duration, Timer};
use esp_println::println;
use leasehund::DhcpServer;

/// Порт TCP-сервера управления.
const PORT: u16 = 8080;

/// Стандартный размер MTU для Ethernet/Wi-Fi.
const WIFI_MTU: usize = 1500;

/// Коэффициент буферизации для сглаживания сетевого джиттера.
const NETWORK_BUFFER_FACTOR: usize = 4;

/// Итоговые размеры буферов сокета, оптимизированные под MTU и размер кадра.
const RX_BUF_SIZE: usize = max_usize(WIFI_MTU, api::MAX_READ_PACKET_SIZE * NETWORK_BUFFER_FACTOR);
const TX_BUF_SIZE: usize = max_usize(WIFI_MTU, api::MAX_WRITE_PACKET_SIZE * NETWORK_BUFFER_FACTOR);

/// Таймаут после неудачной попытки принять подключение.
const ACCEPT_RETRY_TIMEOUT: Duration = Duration::from_secs(10);

const fn max_usize(a: usize, b: usize) -> usize {
    if a > b { a } else { b }
}

pub struct TrafficResources<'a> {
    pos_tx: PosSender,
    cmd_tx: CmdSender<'a>,
    pos_ack_rx: PosAckReceiver,
    cmd_ack_rx: CmdAckReceiver<'a>,
    active_wifi_interface: &'a ActiveWifiInterface,
}

impl<'a> TrafficResources<'a> {
    fn clear(&self) {
        self.pos_tx.clear();
        self.cmd_tx.clear();
        self.pos_ack_rx.clear();
        self.cmd_ack_rx.clear();
    }
}

struct AsyncTrafficResources<'a, M: RawMutex>(embassy_sync::mutex::Mutex<M, TrafficResources<'a>>);

impl<'a, M: RawMutex> AsyncTrafficResources<'a, M> {
    pub fn new(
        pos_tx: PosSender,
        cmd_tx: CmdSender<'a>,
        pos_ack_rx: PosAckReceiver,
        cmd_ack_rx: CmdAckReceiver<'a>,
        active_wifi_interface: &'a ActiveWifiInterface,
    ) -> Self {
        Self(embassy_sync::mutex::Mutex::new(TrafficResources {
            pos_tx,
            cmd_tx,
            pos_ack_rx,
            cmd_ack_rx,
            active_wifi_interface,
        }))
    }

    pub async fn lock(&'a self, interface: WifiInterface) -> TrafficResourcesGuard<'a, M> {
        let res = TrafficResourcesGuard(self.0.lock().await);
        res.active_wifi_interface.signal(interface);
        res
    }
}

pub struct TrafficResourcesGuard<'a, M: RawMutex>(
    embassy_sync::mutex::MutexGuard<'a, M, TrafficResources<'a>>,
);

impl<'a, M: RawMutex> Drop for TrafficResourcesGuard<'a, M> {
    fn drop(&mut self) {
        (*self).0.clear();
        self.active_wifi_interface.signal(WifiInterface::None);
    }
}
impl<'a, M: RawMutex> Deref for TrafficResourcesGuard<'a, M> {
    type Target = TrafficResources<'a>;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl<'a, M: RawMutex> DerefMut for TrafficResourcesGuard<'a, M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

struct Transport<'a> {
    raw: RawTransport<'a>,
    socket_a: TcpSocket<'a>,
    socket_b: TcpSocket<'a>,
}

impl<'a> Transport<'a> {
    fn new(
        radio_interface: WifiInterface,
        stack: Stack<'a>,
        rx_buf_a: &'a mut [u8; RX_BUF_SIZE],
        tx_buf_a: &'a mut [u8; TX_BUF_SIZE],
        rx_buf_b: &'a mut [u8; RX_BUF_SIZE],
        tx_buf_b: &'a mut [u8; TX_BUF_SIZE],
    ) -> Self {
        Self {
            raw: RawTransport::new(radio_interface, stack),
            socket_a: TcpSocket::new(stack, rx_buf_a, tx_buf_a),
            socket_b: TcpSocket::new(stack, rx_buf_b, tx_buf_b),
        }
    }

    async fn run(&'a mut self, transceiver: &'a AsyncTrafficResources<'a, NoopRawMutex>) -> ! {
        self.raw
            .link_scope(transceiver, &mut self.socket_a, &mut self.socket_b)
            .await
    }
}

struct RawTransport<'a> {
    radio_interface: WifiInterface,
    stack: Stack<'a>,
    //transceiver: &'a AsyncTrafficResources<'a, NoopRawMutex>,
}

impl<'a> RawTransport<'a> {
    pub fn new(
        radio_interface: WifiInterface,
        stack: Stack<'a>,
        //transceiver: &'a AsyncTrafficResources<'a, NoopRawMutex>,
    ) -> Self {
        Self {
            radio_interface,
            stack,
            //transceiver,
        }
    }

    async fn link_scope(
        &self,
        transceiver: &'a AsyncTrafficResources<'a, NoopRawMutex>,
        socket_a: &'a mut TcpSocket<'a>,
        socket_b: &'a mut TcpSocket<'a>,
    ) -> ! {
        loop {
            self.stack.wait_link_up().await;
            if let Either::First(_) = select(
                self.stack.wait_link_down(),
                self.config_scope(transceiver, socket_a, socket_b),
            )
            .await
            {
                println!("TRANSPORT: Link lost, resetting hardware...");
                socket_a.abort();
                socket_b.abort();
            }
        }
    }

    async fn config_scope(
        &self,
        transceiver: &'a AsyncTrafficResources<'a, NoopRawMutex>,
        socket_a: &mut TcpSocket<'a>,
        socket_b: &mut TcpSocket<'a>,
    ) {
        loop {
            self.stack.wait_config_up().await;

            // Создаем футуру для DHCP сервера только если это AP
            let dhcp_fut = async {
                if self.radio_interface == WifiInterface::AccessPoint {
                    self.run_dhcp_server().await;
                } else {
                    core::future::pending::<()>().await; // STA просто ждет
                }
            };

            if let Either::First(_) = select(
                self.stack.wait_config_down(),
                select(dhcp_fut, self.tcp_scope(transceiver, socket_a, socket_b)),
            )
            .await
            {
                println!("TRANSPORT: IP Config lost, stopping services...");
                socket_a.abort();
                socket_b.abort();
            }
        }
    }

    async fn run_dhcp_server(&self) -> () {
        let mut dhcp_server: DhcpServer<32, 4> = DhcpServer::new_with_dns(
            Ipv4Addr::new(192, 168, 4, 1),   // Server IP
            Ipv4Addr::new(255, 255, 255, 0), // Subnet mask
            Ipv4Addr::new(192, 168, 4, 1),   // Router/Gateway
            Ipv4Addr::new(8, 8, 8, 8),       // DNS server
            Ipv4Addr::new(192, 168, 4, 2),   // IP pool start
            Ipv4Addr::new(192, 168, 4, 10),  // IP pool end
        );

        // Run the DHCP server (this will loop forever)
        dhcp_server.run(self.stack).await
    }

    async fn tcp_scope(
        &self,
        transceiver: &'a AsyncTrafficResources<'a, NoopRawMutex>,
        socket_a: &mut TcpSocket<'a>,
        socket_b: &mut TcpSocket<'a>,
    ) {
        loop {
            match socket_a.accept(PORT).await {
                Ok(_) => {
                    let mut tr = transceiver.lock(self.radio_interface).await;
                    self.api_scope(socket_a, socket_b, &mut *tr).await;
                }
                Err(e) => {
                    println!("TRANSPORT ERROR: Accept failed: {e:?}");
                    Timer::after(ACCEPT_RETRY_TIMEOUT).await;
                    socket_a.abort();
                }
            }
        }
    }

    async fn api_scope(
        &self,
        active: &mut TcpSocket<'a>,
        spare: &mut TcpSocket<'a>,
        tr: &mut TrafficResources<'a>,
    ) {
        loop {
            if let Some(ep) = active.remote_endpoint() {
                println!("TRANSPORT: client connected: {ep}")
            }

            let (reader, writer) = active.split();
            // Ожидаем либо завершения работы с текущим клиентом, либо нового подключения на запасной сокет.
            match select(
                spare.accept(PORT),
                select(
                    api::send_handle(writer, tr.pos_ack_rx, tr.cmd_ack_rx),
                    api::receive_handle(reader, tr.pos_tx, tr.cmd_tx),
                ),
            )
            .await
            {
                Either::Second(_) => {
                    println!("TRANSPORT: client disconnected");
                    break;
                }
                Either::First(_) => {
                    println!("TRANSPORT: client is displaced by a new one...");
                    mem::swap(active, spare);

                    spare.abort();
                    tr.clear();

                    continue;
                }
            }
        }

        active.abort();
        spare.abort();
        tr.clear();
    }
}

pub struct MultiLinkTransport {
    ap: Transport<'static>,
    sta: Transport<'static>,
}
impl MultiLinkTransport {
    pub fn make(
        ap_stack: Stack<'static>,
        sta_stack: Stack<'static>,
        //transceiver: &'a mut AsyncTrafficResources<NoopRawMutex>,
    ) -> Self {
        let ap_rx_buf_a = mk_static!([u8; RX_BUF_SIZE], [0u8; RX_BUF_SIZE]);
        let ap_tx_buf_a = mk_static!([u8; TX_BUF_SIZE], [0u8; TX_BUF_SIZE]);
        let ap_rx_buf_b = mk_static!([u8; RX_BUF_SIZE], [0u8; RX_BUF_SIZE]);
        let ap_tx_buf_b = mk_static!([u8; TX_BUF_SIZE], [0u8; TX_BUF_SIZE]);

        let sta_rx_buf_a = mk_static!([u8; RX_BUF_SIZE], [0u8; RX_BUF_SIZE]);
        let sta_tx_buf_a = mk_static!([u8; TX_BUF_SIZE], [0u8; TX_BUF_SIZE]);
        let sta_rx_buf_b = mk_static!([u8; RX_BUF_SIZE], [0u8; RX_BUF_SIZE]);
        let sta_tx_buf_b = mk_static!([u8; TX_BUF_SIZE], [0u8; TX_BUF_SIZE]);
        Self {
            ap: Transport::new(
                WifiInterface::AccessPoint,
                ap_stack,
                ap_rx_buf_a,
                ap_tx_buf_a,
                ap_rx_buf_b,
                ap_tx_buf_b,
            ),
            sta: Transport::new(
                WifiInterface::Station,
                sta_stack,
                sta_rx_buf_a,
                sta_tx_buf_a,
                sta_rx_buf_b,
                sta_tx_buf_b,
            ),
        }
    }

    pub async fn run(
        &'static mut self,
        pos_tx: PosSender,
        pos_ack_rx: PosAckReceiver,
        cmd_tx: CmdSender<'static>,
        cmd_ack_rx: CmdAckReceiver<'static>,
        active_wifi_interface: &'static ActiveWifiInterface,
    ) -> ! {
        let tr = mk_static!(
            AsyncTrafficResources<NoopRawMutex>,
            AsyncTrafficResources::new(
                pos_tx,
                cmd_tx,
                pos_ack_rx,
                cmd_ack_rx,
                active_wifi_interface,
            )
        );

        match select(self.sta.run(tr), self.ap.run(tr)).await {
            Either::First(r) | Either::Second(r) => r,
        }
    }
}
