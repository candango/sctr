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

from ..cli import authenticated
from ..services import CtlService, UserService
import argparse
from cartola import config, ftext, sysexits
import firenado.conf
from firenado import service
from firenado.management import ManagementTask
from getpass import getpass
import logging
import os
import sys


logger = logging.getLogger(__name__)
projplat_root = os.path.realpath(os.path.join(
    os.path.dirname(__file__), "..", ".."
))

projplat_app_path = os.path.realpath(os.path.join(
    projplat_root, "projplat"
))

projplat_app_conf_path = os.path.realpath(os.path.join(
    projplat_app_path, "conf",
))

projplat_provisioning = os.path.realpath(os.path.join(
    projplat_root, "provisioning"
))

application = None
data_connected = None


class DataConnectedMixin:

    def __init__(self):
        firenado.conf.log['level'] = config.log_level_from_string("WARNING")
        from firenado.launcher import TornadoLauncher
        # Set logging basic configurations
        if sys.modules[__name__].application is None:
            launcher = TornadoLauncher(dir=firenado.conf.APP_ROOT_PATH)
            launcher.load()
            sys.modules[__name__].application = launcher.application
        self.data_connected = sys.modules[__name__].application
        self.application = self.data_connected

    @property
    def app_component(self):
        return self.application.get_app_component()


class AuthenticatedTask(DataConnectedMixin, ManagementTask):

    def __init__(self, action):
        DataConnectedMixin.__init__(self)
        ManagementTask.__init__(ManagementTask, action=action)
        self._user = None

    def add_arguments(self, parser):
        parser.add_argument("-p", "--password")
        parser.add_argument("-u", "--user")
        parser.add_argument("-s", "--system",
                            action=argparse.BooleanOptionalAction,
                            default=True)

    @property
    def user(self):
        return self._user


class UserListTask(DataConnectedMixin, ManagementTask):

    def __init__(self, action):
        DataConnectedMixin.__init__(self)
        ManagementTask.__init__(self, action=action)

    def add_arguments(self, parser):
        parser.add_argument("-d", "--database", default="motl")
        parser.add_argument("-H", "--host", default="localhost")
        parser.add_argument("-p", "--password")
        parser.add_argument("-P", "--port", default=5432, type=int)
        parser.add_argument("-u", "--user", default=None)

    @service.served_by(UserService)
    def run(self, namespace):
        users = self.user_service.get_users()
        if users is None:
            print("ERROR: No user defined for scrt.")
            sys.exit(1)
        for user in users:
            print(user['name'])
        sys.exit(0)


class UserTestTask(AuthenticatedTask):

    def __init__(self, action):
        AuthenticatedTask.__init__(self, action=action)

    @service.served_by(UserService)
    @authenticated
    def run(self, namespace):
        username = namespace.user
        password = namespace.password
        if username is None:
            username = input("Username:")
        if password is None:
            password = getpass("Password:")
        if self.user_service.authenticate(username, password):
            user = self.user_service.by_username(username)
            print("Success: User %s authenticated." % user['name'])
            sys.exit(sysexits.EX_OK)
        print("ERROR: Invalid user. Please provide valid credentials.")
        sys.exit(sysexits.EX_NOUSER)


class UserAddTask(ManagementTask):

    def add_arguments(self, parser):
        parser.add_argument("-d", "--database", default="motl")
        parser.add_argument("-H", "--host", default="localhost")
        parser.add_argument("-p", "--password")
        parser.add_argument("-P", "--port", default=5432, type=int)
        parser.add_argument("-u", "--user", default="nessie")

    def run(self, namespace):
        raise NotImplementedError


class ListProcessesTask(AuthenticatedTask):

    def __init__(self, action):
        DataConnectedMixin.__init__(self)
        AuthenticatedTask.__init__(self, action=action)

    @authenticated
    @service.served_by(CtlService)
    def run(self, namespace):
        processes = self.ctl_service.get_processes(self.user)
        if len(processes):
            print("Supervisord is running process authorized for %s:\n" %
                  self.user['name'])
            print("%s%s%s%s%s" % (
                ftext.pad("Name", size=34),
                ftext.pad("Status", size=10),
                ftext.pad("pid", size=10),
                ftext.pad("Owner", size=15),
                ftext.pad("Uptime", size=10),
            ))
            for instance in processes:
                print("%s%s%s%s%s" % (
                    ftext.pad(instance['name'], size=34),
                    ftext.pad(instance['status'], size=10),
                    ftext.pad(instance['process'].pid, size=10),
                    ftext.pad(instance['process'].username(), size=15),
                    ftext.pad(instance['uptime'], size=10),
                ))
            sys.exit(sysexits.EX_OK)
        print("INFO: User \"%s\" has no access to running processes in the "
              "supervisor." % self.user['name'])
        sys.exit(sysexits.EX_OK)


class RestartProcessTask(AuthenticatedTask):

    def __init__(self, action):
        DataConnectedMixin.__init__(self)
        AuthenticatedTask.__init__(self, action=action)

    def add_arguments(self, parser):
        super().add_arguments(parser)
        parser.add_argument("process")

    @authenticated
    @service.served_by(CtlService)
    def run(self, namespace):
        counter = 0
        print("Stopping %s: " % namespace.process, end='')
        restarted = False
        for line in self.ctl_service.restart(self.user, namespace.process):
            if counter == 0:
                if line == "stopped":
                    print("[OK]")
                print("Starting %s: " % namespace.process, end='')
            if counter == 1:
                if line == "started":
                    restarted = True
                    print("[OK]")
                    continue
                print("[FAIL]")
            counter += 1
        if restarted:
            sys.exit(sysexits.EX_OK)
        sys.exit(sysexits.EX_UNAVAILABLE)
