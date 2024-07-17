const { config } = require('dotenv');
const Discord = require('discord.js');


config();

const client = new Discord.Client({ intents: [Discord.GatewayIntentBits.Guilds, Discord.GatewayIntentBits.MessageContent] });
const channelToDownload = '844262973043114044';
const fs = require('fs')

client.on('ready',async _=>{
  console.log('Client is ready to download stuff')

  const channel = await client.channels.fetch (channelToDownload)
  const messages = await retriveMessages(channel, 1000)

  const messageAttachments = []
  messages.forEach(message=>{
    const msgId = message[0]
    const msgAttachments = message[1].attachments
    if (msgAttachments.size == 0) return console.log("Message without attachment found");

    console.log("Message with attachment found")

    messageAttachments.push(...msgAttachments.map(attachment => attachment.url))
  
  }) 
  console.log(messageAttachments)
  console.log(`Retrived ${messageAttachments.length} attachments from ${messages.length} messages`)

  fs.writeFileSync('attachments.txt', JSON.stringify(messageAttachments))
})


client.login(process.env.TOKEN);


async function retriveMessages(channel, limit) {
  const messages = [];
  let last_id;

  while (true) {
      const options = { limit: 100 };
      const msgs = await channel.messages.fetch({
        limit: 100,
        before: last_id ? last_id : undefined
      });
      messages.push(...msgs);
      const last_msg = msgs.last();
      if (!last_msg) {
          break;
      }
      last_id = last_msg.id; 
      console.log(`Retrived ${messages.length} messages of ${limit}, last message id: ${last_id}`)
      if (!messages || messages >= limit) {
          break;
      }
  }

  return messages;
}