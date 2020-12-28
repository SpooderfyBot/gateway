import aiohttp
import asyncio
from aiohttp import ClientWebSocketResponse

URL = "ws://127.0.0.1:8080/ws?id=test"


async def responder(ws: ClientWebSocketResponse, fut: asyncio.Future):
    while not ws.closed:
        print(await ws.receive())
    fut.set_result(None)


async def main():
    async with aiohttp.ClientSession() as sess:
        ws = await sess.ws_connect(URL)
        fut = asyncio.get_running_loop().create_future()
        asyncio.create_task(responder(ws, fut))

        await fut

asyncio.run(main())