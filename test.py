#!/usr/bin/env python

from psutil._pslinux import Connections

c = Connections()
print("TCP Connections: {}".format(c.retrieve("tcp")))
