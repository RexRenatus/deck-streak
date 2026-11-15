# SPEC-622 red-first record

A1, A3, A4, A5 and A6 are red at the red commit, which adds the test module alone, each by an
assertion on the text the front door and the index lack, and green at the commit that writes the
four pages and the rows, with the tests unchanged. A2 is `not red`: every relative link in the
living docs already resolved at the cut, so it guards them, and its planted refusals (a missing
file, a missing folder, a missing anchor) and its ignored fence and code span are inside the test.
The reds were measured locally by running each criterion's command on the red commit's tree, and
read again by name in the pull request's own run.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_the_docs_index_links_every_living_doc_and_record_folder
A2: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_every_relative_link_in_a_living_doc_resolves
A3: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_contributing_states_the_branch_model_and_the_queue_file_facts
A4: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_the_readme_and_agents_name_the_merge_queue_and_its_label
A5: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_the_front_door_names_the_web_app_and_the_iphone_and_ipad_app
A6: python3 -m unittest discover -s scripts/tests -p test_front_door_docs.py -k test_the_front_door_names_the_mini_app_and_bot_only_as_deprecated
```

```red-first
A2: not red: every relative link in the living docs resolved at the cut; it guards them, with its planted refusals inside the test
```
