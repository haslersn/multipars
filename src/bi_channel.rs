use async_bincode::tokio::{AsyncBincodeReader, AsyncBincodeWriter};
use async_bincode::AsyncDestination;

use crate::connection::{Connection, StreamError};

pub struct BiChannel<Message> {
    pub reader: AsyncBincodeReader<quinn::RecvStream, Message>,
    pub writer: AsyncBincodeWriter<quinn::SendStream, Message, AsyncDestination>,
}

impl<Message> BiChannel<Message> {
    pub async fn open(
        conn: &mut Connection,
        name: &str,
    ) -> Result<BiChannel<Message>, StreamError> {
        let (tx, rx) = conn.open_bi(name).await?;
        Ok(BiChannel {
            reader: AsyncBincodeReader::from(rx),
            writer: AsyncBincodeWriter::from(tx).for_async(),
        })
    }

    pub fn split(
        &mut self,
    ) -> (
        &mut AsyncBincodeReader<quinn::RecvStream, Message>,
        &mut AsyncBincodeWriter<quinn::SendStream, Message, AsyncDestination>,
    ) {
        (&mut self.reader, &mut self.writer)
    }
}
