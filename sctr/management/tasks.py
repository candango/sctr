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
from ..services import UserService
import argparse
from cartola import config, sysexits
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


class AuthenticatedTask(ManagementTask):

    def add_arguments(self, parser):
        parser.add_argument("-p", "--password")
        parser.add_argument("-u", "--user")
        parser.add_argument("-s", "--system",
                            action=argparse.BooleanOptionalAction,
                            default=False)


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


class UserTestTask(DataConnectedMixin, AuthenticatedTask):

    def __init__(self, action):
        DataConnectedMixin.__init__(self)
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


class ListProcessesTask(ManagementTask):

    def add_arguments(self, parser):
        parser.add_argument("-p", "--password")
        parser.add_argument("-u", "--user")

    def run(self, namespace):
        username = input("Username:")
        password = getpass("Password:")
        print(username, password)
