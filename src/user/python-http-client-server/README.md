# `python-http-client-server`

Currently, this app fails w/:
```
Traceback (most recent call last):
  File "/home/danbugs/repos/nanvix/./src/user/python-http-client-server/__main__.py", line 3, in <module>
    import http.server
  File "/home/danbugs/repos/nanvix/sysroot-debug/lib/python3.12/http/server.py", line 92, in <module>
    import email.utils
  File "/home/danbugs/repos/nanvix/sysroot-debug/lib/python3.12/email/utils.py", line 29, in <module>
    import socket
  File "/home/danbugs/repos/nanvix/sysroot-debug/lib/python3.12/socket.py", line 52, in <module>
    import _socket
```