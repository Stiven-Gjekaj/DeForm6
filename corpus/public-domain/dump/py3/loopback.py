# loopback.py by Jigsy (https://github.com/Jigsy1) released under the Unlicense.

from _thread import *

import socket

def accept_connection(client, address):
    print("Accepted: {}".format(client))
    while True:
        data = client.recv(4096).decode("UTF-8")
        if not data:
            break
        print(data)
    print("Closed: {}".format(client))
    client.close()

LOOPBACK_HOST = "127.0.0.1"
LOOPBACK_PORT = 80

loopback = socket.socket()

try:
    loopback.bind((LOOPBACK_HOST, LOOPBACK_PORT))
except socket.error as bindErr:
    print("Error: {}".format(str(bindErr)))

loopback.listen()
print("Listening on port {}...".format(LOOPBACK_PORT))

while True:
    client, address = loopback.accept()
    start_new_thread(accept_connection, (client, address))
loopback.close()

# EOF
