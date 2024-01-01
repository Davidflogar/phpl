<?php
trait A
{
	public function bigTalk()
	{
		echo 'A';
	}
}

trait B
{
	public function bigTalk()
	{
		echo 'B';
	}
}

class Aliased_Talker
{
	use A;
	use B;
}
