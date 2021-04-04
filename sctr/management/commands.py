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
from cartola import ftext
from firenado.management import ManagementCommand
from tornado import template
import os

SCTR_ROOT = os.path.realpath(
    os.path.join(os.path.dirname(__file__), ".."))

loader = template.Loader(os.path.join(SCTR_ROOT, "templates", "management"))


class SctrManagementCommand(ManagementCommand):

    def __init__(self, name, description, cmd_help, **kwargs):
        super(SctrManagementCommand, self).__init__(
            name, description, cmd_help, **kwargs)
        self.help = loader.load("sctr_command_help.txt").generate(
            command=self)

    def get_help_description(self):
        return "%s %s" % (
            ftext.pad(self.name, size=20),
            ftext.columnize(self.description, columns=30,
                            newline="\n%s" % (" " * 27)))

    def get_subcommands_help(self):
        subcommands = []
        if self.sub_commands is not None:
            subcommands = self.sub_commands
        return loader.load("subcommands.txt").generate(
            subcommands=subcommands)


sctrProcessSubcommands = [
    SctrManagementCommand("list", "List Process", "",
                          tasks=tasks.ListProcessesTask),
]

sctrUserSubcommands = [
    SctrManagementCommand("add", " Add User", "", tasks=tasks.UserAddTask),
    SctrManagementCommand("list", "List Users", "", tasks=tasks.UserListTask),
    SctrManagementCommand("test", "Test User", "", tasks=tasks.UserTestTask),
]

sctrSubcommands = [
    CstrManagementCommand(
        "proc",
        "Process related tasks",
        "",
        sub_commands=sctrProcessSubcommands),
    SctrManagementCommand(
        "user",
        "User related tasks",
        "",
        sub_commands=sctrUserSubcommands),
]

SctrManagementCommand(
    "sctr",
    "Supervisord Controller related commands",
    "",
    category="Supervisord Controller",
    sub_commands=sctrSubcommands)
