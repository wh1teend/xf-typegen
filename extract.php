<?php

use XF\App;
use XF\Mvc\Entity\Entity;
use XF\Util\Php;

error_reporting(E_ALL & ~E_DEPRECATED & ~E_USER_DEPRECATED);
ini_set('display_errors', 'stderr');

defined('STDIN') || define('STDIN', fopen('php://stdin', 'r'));
defined('STDOUT') || define('STDOUT', fopen('php://stdout', 'w'));
defined('STDERR') || define('STDERR', fopen('php://stderr', 'w'));

const XF_TYPEGEN_GENERATOR = 'xf-typegen-standalone 0.1.0';

$opts = parse_args(array_slice($argv, 1));

$root = $opts['root'] ?: (getenv('XF_ROOT') ?: null);
if (!$root || !is_dir($root) || !file_exists($root . '/src/XF.php'))
{
	fwrite(STDERR, "Error: pass a valid XenForo root as the first argument (or set XF_ROOT).\n");
	fwrite(STDERR, "Usage: php extract.php /path/to/xenforo/root [--out=FILE] [--addon=ID] [--minify]\n");
	exit(1);
}

$app = boot_xf($root);
$extractor = new Extractor($app);

$errors = [];
$data = $extractor->extract(['addon' => $opts['addon']], $errors);

write_contract($data, $opts['out'], $opts['minify']);
report($data, $errors);

if ($opts['mixin'] !== null)
{
	$remove = ($opts['mixin'] === 'remove');
	$stats = $extractor->injectMixins($opts['addon'], $remove);
	fwrite(STDERR, sprintf(
		"%s @mixin in %d file(s)%s.\n",
		$remove ? 'Removed' : 'Applied',
		$stats['changed'],
		$stats['skipped'] ? sprintf(" (%d not writable, skipped)", $stats['skipped']) : ''
	));
}

function parse_args(array $args)
{
	$opts = ['root' => null, 'addon' => null, 'minify' => false, 'out' => null, 'mixin' => null];

	foreach ($args as $arg)
	{
		if (strpos($arg, '--addon=') === 0)
		{
			$opts['addon'] = substr($arg, strlen('--addon='));
		}
		else if (strpos($arg, '--out=') === 0)
		{
			$opts['out'] = substr($arg, strlen('--out='));
		}
		else if ($arg === '--minify')
		{
			$opts['minify'] = true;
		}
		else if ($arg === '--mixin')
		{
			$opts['mixin'] = 'apply';
		}
		else if (strpos($arg, '--mixin=') === 0)
		{
			$opts['mixin'] = substr($arg, strlen('--mixin='));
		}
		else if ($opts['root'] === null && strpos($arg, '--') !== 0)
		{
			$opts['root'] = $arg;
		}
	}

	return $opts;
}

function boot_xf($root)
{
	require $root . '/src/XF.php';

	XF::start($root);

	set_error_handler(function ($errno, $errstr, $errfile, $errline)
	{
		if ($errno & (E_DEPRECATED | E_USER_DEPRECATED | 2048))
		{
			return true;
		}
		return XF::handlePhpError($errno, $errstr, $errfile, $errline);
	});

	return XF::setupApp(\XF\Pub\App::class);
}

function write_contract(array $data, $out, $minify)
{
	$flags = JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE;
	if (!$minify)
	{
		$flags |= JSON_PRETTY_PRINT;
	}

	$json = json_encode($data, $flags) . "\n";

	if ($out === null)
	{
		echo $json;
		return;
	}

	if (@file_put_contents($out, $json) === false)
	{
		fwrite(STDERR, "Error: could not write to $out\n");
		exit(1);
	}
	fwrite(STDERR, "Wrote $out\n");
}

function report(array $data, array $errors)
{
	fwrite(STDERR, sprintf("Extracted %d entities.\n", count((array) $data['entities'])));

	if ($errors)
	{
		fwrite(STDERR, sprintf("Skipped %d entit%s:\n", count($errors), count($errors) === 1 ? 'y' : 'ies'));
		foreach ($errors as $short => $message)
		{
			fwrite(STDERR, "  - $short: $message\n");
		}
	}
}

class Extractor
{
	const TYPE_MAP = [
		Entity::INT              => ['int',         'int'],
		Entity::UINT             => ['int',         'uint'],
		Entity::FLOAT            => ['float',       'float'],
		Entity::BOOL             => ['bool',        'bool'],
		Entity::STR              => ['string',      'str'],
		Entity::BINARY           => ['string',      'binary'],
		Entity::SERIALIZED       => ['array|bool',  'serialized'],
		Entity::SERIALIZED_ARRAY => ['array',       'serialized_array'],
		Entity::JSON             => ['array|null',  'json'],
		Entity::JSON_ARRAY       => ['array',       'json_array'],
		Entity::LIST_LINES       => ['array',       'list_lines'],
		Entity::LIST_COMMA       => ['array',       'list_comma'],
		Entity::LIST_ARRAY       => ['array',       'list_array'],
	];

	protected $app;

	public function __construct(App $app)
	{
		$this->app = $app;
	}

	public function extract(array $options = [], array &$errors = [])
	{
		$addOnFilter = $options['addon'] ?? null;

		$shortNames = $this->discoverEntityShortNames($addOnFilter);
		sort($shortNames, SORT_STRING | SORT_FLAG_CASE);

		$entities = [];
		foreach ($shortNames as $shortName)
		{
			try
			{
				$entity = $this->buildEntity($shortName);
				if ($entity !== null)
				{
					$entities[$shortName] = $entity;
				}
			}
			catch (\Throwable $ex)
			{
				$errors[$shortName] = $ex->getMessage();
			}
		}

		return [
			'version' => 1,
			'generator' => XF_TYPEGEN_GENERATOR,
			'generatedAt' => date('c', \XF::$time),
			'xf' => $this->getXfInfo(),
			'activeAddOns' => $this->getActiveAddOnIds(),
			'classExtensions' => $this->getClassExtensions(),
			'services' => $this->emptyToObject($this->discoverServices($addOnFilter)),
			'captchas' => $this->emptyToObject($this->discoverCaptchas($addOnFilter)),
			'options' => $this->emptyToObject($this->getOptions()),
			'entities' => $this->emptyToObject($entities),
		];
	}

	protected function getClassExtensions()
	{
		try
		{
			$map = $this->app->container('extension.classExtensions');
		}
		catch (\Throwable $e)
		{
			$map = [];
		}

		if (!is_array($map))
		{
			return [];
		}

		$extensions = [];
		foreach ($map as $fromClass => $extendClasses)
		{
			$prev = ltrim($fromClass, '\\');

			foreach ((array) $extendClasses as $extendClass)
			{
				$extendClass = ltrim($extendClass, '\\');
				if ($extendClass === '')
				{
					continue;
				}

				$nsSplit = strrpos($extendClass, '\\');
				$proxy = ($nsSplit !== false)
					? substr($extendClass, 0, $nsSplit) . '\\XFCP_' . substr($extendClass, $nsSplit + 1)
					: 'XFCP_' . $extendClass;

				$extensions[] = ['proxy' => $proxy, 'extends' => $prev];
				$prev = $extendClass;
			}
		}

		usort($extensions, function ($a, $b) { return strcmp($a['proxy'], $b['proxy']); });

		return $extensions;
	}

	protected function emptyToObject(array $array)
	{
		return $array ?: new \stdClass();
	}

	protected function getOptions()
	{
		$options = [];

		foreach ($this->app->options() as $name => $value)
		{
			if (!is_string($name) || $name === '')
			{
				continue;
			}

			switch (gettype($value))
			{
				case 'boolean': $type = 'bool'; break;
				case 'integer': $type = 'int'; break;
				case 'double': $type = 'float'; break;
				case 'string': $type = 'string'; break;
				case 'array': $type = 'array'; break;
				default: $type = 'mixed'; break;
			}

			$options[$name] = $type;
		}

		ksort($options, SORT_STRING | SORT_FLAG_CASE);

		return $options;
	}

	public function injectMixins($addOnFilter = null, $remove = false)
	{
		$changed = 0;
		$skipped = 0;

		foreach ($this->discoverEntityShortNames($addOnFilter) as $short)
		{
			$base = \XF::stringToClass($short, '%s\Entity\%s');
			if (!class_exists($base))
			{
				continue;
			}

			$ref = new \ReflectionClass($base);
			if (!$ref->isInstantiable() || !$ref->isSubclassOf(Entity::class))
			{
				continue;
			}

			$mixin = '\\XFIDEHelper\\Entity_' . self::ident($short);

			switch ($this->applyMixin($ref, $mixin, $remove))
			{
				case 'changed': $changed++; break;
				case 'skip': $skipped++; break;
			}
		}

		return ['changed' => $changed, 'skipped' => $skipped];
	}

	protected function applyMixin(\ReflectionClass $ref, $mixin, $remove)
	{
		$file = $ref->getFileName();
		if (!$file || !is_writable($file))
		{
			return 'skip';
		}

		$contents = file_get_contents($file);
		if ($contents === false)
		{
			return 'skip';
		}

		$existing = $ref->getDocComment();
		$tag = '@mixin ' . $mixin;

		if ($remove)
		{
			if ($existing === false || strpos($existing, $tag) === false)
			{
				return 'unchanged';
			}

			$stripped = preg_replace('#\R[ \t]*\*[ \t]*' . preg_quote($tag, '#') . '[ \t]*#', '', $existing, 1);

			if (preg_match('#^/\*\*\s*\*/$#', $stripped))
			{
				$contents = str_replace($existing . "\n", '', $contents, $count);
				if ($count === 0)
				{
					$contents = str_replace($existing, '', $contents);
				}
			}
			else
			{
				$contents = str_replace($existing, $stripped, $contents);
			}
		}
		else
		{
			if ($existing !== false && strpos($existing, $tag) !== false)
			{
				return 'unchanged';
			}

			if ($existing !== false)
			{
				$updated = preg_replace('#\s*\*/\s*$#', "\n * " . $tag . "\n */", $existing, 1);
				$contents = str_replace($existing, $updated, $contents);
			}
			else
			{
				$anchor = 'class ' . $ref->getShortName() . ' extends ';
				$pos = strpos($contents, $anchor);
				if ($pos === false)
				{
					return 'skip';
				}
				$contents = substr($contents, 0, $pos) . "/**\n * " . $tag . "\n */\n" . substr($contents, $pos);
			}
		}

		return file_put_contents($file, $contents) !== false ? 'changed' : 'skip';
	}

	protected static function ident($short)
	{
		return preg_replace('/[^A-Za-z0-9_]/', '_', $short);
	}

	protected function sourceBases($addOnFilter = null)
	{
		$bases = [];

		if ($addOnFilter === null || $addOnFilter === 'XF')
		{
			$bases['XF'] = \XF::getSourceDirectory() . \XF::$DS . 'XF';
		}

		$addOnManager = $this->app->addOnManager();
		foreach ($addOnManager->getInstalledAddOns() as $addOn)
		{
			$addOnId = $addOn->getAddOnId();
			if ($addOnId === 'XF')
			{
				continue;
			}
			if ($addOnFilter !== null && $addOnFilter !== $addOnId)
			{
				continue;
			}

			$bases[$addOnId] = $addOnManager->getAddOnPath($addOnId);
		}

		return $bases;
	}

	public function discoverEntityShortNames($addOnFilter = null)
	{
		$shortNames = [];

		foreach ($this->sourceBases($addOnFilter) as $addOnId => $base)
		{
			$path = $base . \XF::$DS . 'Entity';
			$shortNames = array_merge($shortNames, $this->scanClassDirectory($path, $addOnId));
		}

		return array_values(array_unique($shortNames));
	}

	protected function discoverServices($addOnFilter = null)
	{
		return $this->discoverClasses($addOnFilter, 'Service', 'XF\Service\AbstractService');
	}

	protected function discoverCaptchas($addOnFilter = null)
	{
		return $this->discoverClasses($addOnFilter, 'Captcha', 'XF\Captcha\AbstractCaptcha');
	}

	protected function discoverClasses($addOnFilter, $infix, $baseClass)
	{
		$classes = [];

		foreach ($this->sourceBases($addOnFilter) as $addOnId => $base)
		{
			$path = $base . \XF::$DS . $infix;
			foreach ($this->scanClassDirectory($path, $addOnId) as $short)
			{
				$class = \XF::stringToClass($short, '%s\\' . $infix . '\%s');
				if (!class_exists($class))
				{
					continue;
				}

				try
				{
					$reflection = new \ReflectionClass($class);
				}
				catch (\Throwable $e)
				{
					continue;
				}

				if ($reflection->isInstantiable() && $reflection->isSubclassOf($baseClass))
				{
					$classes[$short] = ltrim($class, '\\');
				}
			}
		}

		ksort($classes, SORT_STRING | SORT_FLAG_CASE);

		return $classes;
	}

	protected function scanClassDirectory($path, $addOnId)
	{
		if (!file_exists($path) || !is_dir($path))
		{
			return [];
		}

		$shortNames = [];
		$vendor = str_replace('/', '\\', $addOnId);

		$iterator = new \RegexIterator(
			\XF\Util\File::getRecursiveDirectoryIterator($path, null, null),
			'/\.php$/'
		);

		foreach ($iterator as $file)
		{
			$name = str_replace('.php', '', $file->getFilename());

			if (strpos($name, 'XFCP_') === 0)
			{
				continue;
			}

			$subDir = substr($file->getPath(), strlen($path));
			$subDir = ltrim(str_replace('/', '\\', $subDir) . '\\', '\\');

			$shortNames[] = $vendor . ':' . $subDir . $name;
		}

		return $shortNames;
	}

	public function buildEntity($shortName)
	{
		$em = $this->app->em();

		$baseClass = \XF::stringToClass($shortName, '%s\Entity\%s');
		if (!class_exists($baseClass))
		{
			return null;
		}

		$className = $em->getEntityClassName($shortName);

		$reflection = new \ReflectionClass($className);
		if (!$reflection->isInstantiable() || !$reflection->isSubclassOf(Entity::class))
		{
			return null;
		}

		$structure = $em->getEntityStructure($shortName);

		$primaryKey = array_values((array) $structure->primaryKey);
		$primaryKeyLookup = array_flip($primaryKey);

		$entity = [
			'shortName' => $shortName,
			'class' => ltrim($className, '\\'),
		];

		if ($structure->table)
		{
			$entity['table'] = $structure->table;
		}
		if ($structure->contentType)
		{
			$entity['contentType'] = $structure->contentType;
		}
		if ($primaryKey)
		{
			$entity['primaryKey'] = $primaryKey;
		}

		$entity['finder'] = $this->resolveSibling($shortName, '%s\Finder\%s');
		$entity['repository'] = $this->resolveSibling($shortName, '%s\Repository\%s');

		$entity['columns'] = $this->emptyToObject($this->buildColumns($structure, $primaryKeyLookup));
		$entity['relations'] = $this->emptyToObject($this->buildRelations($structure));
		$entity['getters'] = $this->emptyToObject($this->buildGetters($structure, $reflection));

		return $entity;
	}

	protected function buildColumns($structure, array $primaryKeyLookup)
	{
		$columns = [];

		foreach ($structure->columns as $name => $def)
		{
			$type = $def['type'] ?? null;
			$mapped = ($type !== null) ? (self::TYPE_MAP[$type] ?? null) : null;

			$column = [
				'phpType' => !empty($def['typeHint']) ? $def['typeHint'] : ($mapped[0] ?? 'mixed'),
				'nullable' => !empty($def['nullable']),
				'xfType' => $mapped[1] ?? 'unknown',
			];

			if (isset($primaryKeyLookup[$name]))
			{
				$column['primary'] = true;
			}

			$columns[$name] = $column;
		}

		return $columns;
	}

	protected function buildRelations($structure)
	{
		$relations = [];

		foreach ($structure->relations as $name => $def)
		{
			$entityShort = $def['entity'] ?? null;
			if (!$entityShort)
			{
				continue;
			}

			$class = \XF::stringToClass($entityShort, '%s\Entity\%s');

			$relations[$name] = [
				'to' => (($def['type'] ?? null) === Entity::TO_MANY) ? 'many' : 'one',
				'entity' => $entityShort,
				'class' => class_exists($class) ? ltrim($class, '\\') : null,
			];
		}

		return $relations;
	}

	protected function buildGetters($structure, \ReflectionClass $reflection)
	{
		$getters = [];

		foreach ($structure->getters as $name => $def)
		{
			if ($def === false)
			{
				continue;
			}

			if (is_array($def) && isset($def['getter']) && is_string($def['getter']))
			{
				$methodName = $def['getter'];
			}
			else
			{
				$methodName = 'get' . ucfirst(Php::camelCase($name));
			}

			if (!$reflection->hasMethod($methodName))
			{
				continue;
			}

			$getters[$name] = $this->resolveGetterType($reflection->getMethod($methodName));
		}

		return $getters;
	}

	protected function resolveGetterType(\ReflectionMethod $method)
	{
		$comment = $method->getDocComment();
		if ($comment && preg_match('/^\s*?\*\s*?@return\s+(\S+)/mi', $comment, $matches))
		{
			$type = trim($matches[1]);
			return [
				'phpType' => $type,
				'nullable' => stripos($type, 'null') !== false,
				'source' => 'phpdoc',
			];
		}

		$returnType = $method->getReturnType();
		if ($returnType)
		{
			$resolved = $this->stringifyReturnType($returnType);
			if ($resolved['phpType'] !== '')
			{
				$resolved['source'] = 'native';
				return $resolved;
			}
		}

		return ['phpType' => 'mixed', 'nullable' => false, 'source' => 'unknown'];
	}

	protected function stringifyReturnType(\ReflectionType $returnType)
	{
		$parts = ($returnType instanceof \ReflectionUnionType) ? $returnType->getTypes() : [$returnType];

		$types = [];
		$nullable = false;

		foreach ($parts as $part)
		{
			$partName = $part->getName();
			if ($partName === 'null')
			{
				$nullable = true;
				continue;
			}

			$types[] = ($part->isBuiltin() ? '' : '\\') . $partName;

			if ($part->allowsNull())
			{
				$nullable = true;
			}
		}

		$type = implode('|', $types);
		if ($nullable && $type !== '')
		{
			$type .= '|null';
		}

		return ['phpType' => $type, 'nullable' => $nullable];
	}

	protected function resolveSibling($shortName, $formatter)
	{
		$class = \XF::stringToClass($shortName, $formatter);
		return class_exists($class) ? ltrim($class, '\\') : null;
	}

	protected function getXfInfo()
	{
		$versionId = \XF::$versionId;

		return [
			'versionId' => $versionId,
			'version' => sprintf('%d.%d.%d',
				(int) ($versionId / 1000000),
				(int) ($versionId / 10000) % 100,
				(int) ($versionId / 100) % 100
			),
			'branch' => sprintf('%d.%d',
				(int) ($versionId / 1000000),
				(int) ($versionId / 10000) % 100
			),
		];
	}

	protected function getActiveAddOnIds()
	{
		$ids = ['XF'];

		foreach ($this->app->addOnManager()->getInstalledAddOns() as $addOn)
		{
			if ($addOn->getAddOnId() !== 'XF' && $addOn->isActive())
			{
				$ids[] = $addOn->getAddOnId();
			}
		}

		$ids = array_values(array_unique($ids));
		sort($ids, SORT_STRING | SORT_FLAG_CASE);

		return $ids;
	}
}
