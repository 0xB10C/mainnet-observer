#!/bin/bash

hugo new charts/${1}.md
touch static/js/charts/${1}.js
vim static/js/charts/${1}.js
