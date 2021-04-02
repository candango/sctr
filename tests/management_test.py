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
from tests import chdir_fixture_app
import unittest


class ManagementTestCase(unittest.TestCase):
    """ Test case testing the management tasks provided by sctr
    """

    def test_data_connected_mixin(self):
        """ Test if DataConnectedMixin will load a valid sctr firenado app with
        it's sctr.yml loaded.
        """
        sctr_username = "sctr_user"
        chdir_fixture_app("sctr_instance")
        from sctr.management.tasks import DataConnectedMixin
        data_connected = DataConnectedMixin()
        app_component = data_connected.application.get_app_component()
        self.assertEqual(sctr_username, app_component.conf['users'][0]['name'])
