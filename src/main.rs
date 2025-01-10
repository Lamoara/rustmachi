use std::{io::Error, net::SocketAddr, sync::Arc, time::Duration};

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, time::timeout};
use wintun::{Adapter, Session};

#[tokio::main]
async fn main() 
{
    let session = Arc::new(create_tun_interface().unwrap());

    let listen_addr: SocketAddr = "0.0.0.0:8080".parse().unwrap(); //Replace witth proper setup
    let target_addr: SocketAddr = "0.0.0.0:8080".parse().unwrap(); //Replace witth proper setup

    let listener = TcpListener::bind(listen_addr.clone()).await.unwrap(); //Replace with proper setup
    let stream = TcpStream::connect(target_addr.clone()).await.unwrap(); //Replace with proper setup

    let listener_task = tokio::task::spawn(
    {
        let session = Arc::clone(&session);
        let stream = stream;

        async move 
        {
            listener_task(session, stream).await.unwrap();
        }
    }); 

    let sender_task = tokio::task::spawn(
    {
        let session = Arc::clone(&session);
        let listener = listener;

        async move 
        {
            sender_task(session, listener, target_addr).await.unwrap();
        }
    }); 

    listener_task.await.unwrap();
    sender_task.await.unwrap();
}

fn create_tun_interface() -> Result<Session, Error> 
{
    let wintun = unsafe { wintun::load_from_path("C:/Users/jorge/Downloads/wintun-0.14.1/wintun/bin/amd64/wintun.dll") }?;

    let adapter = match Adapter::open(&wintun, "Rustmachi") {
        Ok(a) => a,
        Err(_) => Adapter::create(&wintun, "Rustmachi", "Rustmachi", None)?
    }; 

    let session = adapter.start_session(wintun::MAX_RING_CAPACITY)?;

    Ok(session)
}

async fn listener_task(session: Arc<Session>, mut stream: TcpStream) -> Result<(),Error>
{
    loop
    {
        let packet = session.receive_blocking()?;
        let buf = packet.bytes();

        stream.writable().await?;
        stream.write(buf).await.unwrap();
    }
}

async fn sender_task(session: Arc<Session>, listener: TcpListener, target_addr: SocketAddr) -> Result<(),Error>
{
    loop
    {
        
        let (mut stream, addr) = listener.accept().await?;

        let timeout_duration = 5;
        loop {
            let mut packet = session.allocate_send_packet(1024)?;
            //Tokio timeout allows to listen for a certain period of time
            match timeout(Duration::from_secs(timeout_duration), stream.read(packet.bytes_mut())).await{
                //Connection has been closed externally
                Ok(Ok(n)) if n == 0 => {
                    println!("Connection closed");
                    break;
                }
                //Data recieved
                Ok(Ok(n)) => {
                    println!("Read {} bytes: {:?}", n, packet.bytes());
    
                    if addr != target_addr
                    {
                        println!("Cosa rara");
                        continue;
                    }
            
                    println!("Guay");
                    session.send_packet(packet);
                }
                //Error reading stream
                Ok(Err(e)) => {
                    println!("Failed to read from stream: {}", e);
                    break;
                }
                //Timed out
                Err(_) => {
                    println!("Read timed out");
                    stream.shutdown().await.unwrap_or_else(|e| {eprint!("Failed to shutdown connection: {e}")});
                    break;
                }
            }
        }

    }
}
