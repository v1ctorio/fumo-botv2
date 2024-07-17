const { config } = require('dotenv');
const Discord = require('discord.js');


config();

const client = new Discord.Client({ intents: [Discord.GatewayIntentBits.Guilds, Discord.GatewayIntentBits.MessageContent] });
const channelToDownload = '844262973043114044';

client.on('ready',async _=>{
  console.log('Client is ready to download stuff')

  const channel = await client.channels.fetch (channelToDownload)
  const messages = await channel.messages.fetch()
  messages.forEach(message=>{
    console.log(message.content)
  })
})



client.login(process.env.TOKEN);
