// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

import { attr, FASTElement } from "@microsoft/fast-element";
import { declarativeTemplate } from "@microsoft/fast-element/declarative.js";

export class HelloWorld extends FASTElement {
    @attr greeting: string = "Hello world";
}

void HelloWorld.define({
    name: "hello-world",
    template: declarativeTemplate(),
});
