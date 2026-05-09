// Copyright (c) Microsoft Corporation.
// Licensed under the MIT license.

import { attr, FASTElement } from "@microsoft/fast-element";
import { declarativeTemplate } from "@microsoft/fast-element/declarative.js";

export class HelloWorld extends FASTElement {
    @attr greeting: string = "Hello world";

    handleButtonPress(): void {
        alert("button pressed!");
    }
}

void HelloWorld.define({
    name: "hello-world",
    template: declarativeTemplate(),
});
