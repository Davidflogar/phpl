<?php

class Hello {
	public function sayHello(string $name) {
		echo "Hello " . $name;
	}
}

$hello = new Hello();
$hello->sayHello("John");
