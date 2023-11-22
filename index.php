<?php

class MyHelloWorld {
    use A, B {
		B::example insteadof A;
		A::example as private exampleA;
	}
}
