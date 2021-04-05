#!/usr/bin/env python
#
# Copyright 2021 Flavio Garcia
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#    http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

from cartola import security
from firenado import service
import pexpect
from psutil import Process


class DataConnectedMixin:

    @property
    def app_component(self):
        return self.data_connected.get_app_component()


class CtlService(DataConnectedMixin, service.FirenadoService):

    @property
    def ctl_cmd(self):
        return self.app_component.conf['supervisor']['ctl']

    def get_processes(self, user):
        processes = []
        status = pexpect.run("%s status" % self.ctl_cmd).decode()
        for line in status.split("\r\n")[:-1]:
            denied = False
            properties = ['details', 'status', 'name']
            instance = {
                'name': None,
                'status': None,
                'process': None,
                'uptime': None,
                'time': None
            }
            for item in filter(lambda field: field.strip() != "",
                               line.split("  ")):
                prop = properties.pop()
                if prop == 'details':
                    details = item.strip().split(",")
                    pid = int(details[0].split(" ")[1])
                    process = Process(pid=pid)
                    if process.username() != user['name']:
                        denied = True
                        continue
                    instance['process'] = Process(pid=pid)
                    instance['uptime'] = details[1].strip().split(" ")[1]
                    if len(details) > 2:
                        instance['time'] = details[2]
                    break
                instance[prop] = item.strip()
            if not denied:
                processes.append(instance)
        return processes

    def restart(self, user, process):
        current = None
        for instance in self.get_processes(user):
            if instance['name'] == process:
                current = instance
        if current is None:
            return []
        try:
            restart = pexpect.spawn("%s restart %s" % (self.ctl_cmd, process))
            while True:
                restart.expect("%s: (.*)\r\n" % process)
                yield restart.match.group(1).decode()
        except pexpect.EOF:
            return

class UserService(DataConnectedMixin, service.FirenadoService):

    def by_username(self, username):
        if "users" in self.app_component.conf:
            for user in self.app_component.conf['users']:
                if user['name'] == username:
                    return user
        return None

    def authenticate(self, username, password):
        user = self.by_username(username)
        if user is None:
            return False
        manager = security.Sha512KeyManager()
        return manager.validate(password, user['password'])

    def get_users(self):
        if "users" in self.app_component.conf:
            return self.app_component.conf['users']
        return None
