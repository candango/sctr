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

from . import tasks
from firenado.management import ManagementCommand
from tornado import template
import os

SCTR_ROOT = os.path.realpath(
    os.path.join(os.path.dirname(__file__), ".."))

loader = template.Loader(os.path.join(SCTR_ROOT, "templates", "management"))

sctrUserSubcommands = [
    ManagementCommand("add", " Add User", "", tasks=tasks.UserAddTask),
    ManagementCommand("list", "List Users", "", tasks=tasks.UserListTask),
]


sctrSubcommands = [
    ManagementCommand(
        "user",
        "User related tasks",
        loader.load("sctr_user_command_help.txt").generate(
            subcommands=sctrUserSubcommands),
        sub_commands=sctrUserSubcommands),
]

ManagementCommand(
    "sctr",
    "Supervisord Controller related commands",
    loader.load("sctr_command_help.txt").generate(subcommands=sctrSubcommands),
    category="Supervisord Controller",
    sub_commands=sctrSubcommands)
