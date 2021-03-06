# Relation

用于实现目标之间的转换，其定义存储到数据表 relation 中

## 存储 `Relation`

示例如下：

```sql
INSERT INTO relation
(from_meta, to_meta, settings)
VALUES('B:sale/order:1', 'B:sale/orderState:1', '{"target_states":{"add":["new"]}}');
```

## 定义 `Relation` 的处理方式

上面的示例 SQL 中的 settings 字段用于对每个关系的处理方式进行个性化定义。settings 的值是 JSON形式的 `RelationSettings` 对象，其结构如下。

```rust
pub struct RelationSettings {
    pub selector: Option<FlowSelector>,
    pub executor: Option<Executor>,
    pub filter_before: Vec<Executor>,
    pub filter_after: Vec<Executor>,
    pub use_upstream_id: bool,
    pub target: RelationTarget,
    pub delay: i32,
    pub delay_on_para: (i32, u8),
    pub id_bridge: bool,
}
```

- selector：属性用于选择符合条件的 `Instance` 进入 `Executor` 进入处理，其结构见下方 `FlowSelector`的结构说明。
- executor：属性用于定义谁来做这个转化处理，其结构见下方 `Executor`的结构说明。
- filter_before: 在executor之前执行用于对输入实例进行修正。可以是多个，按给定的顺序执行。
- filter_after: 在executor之后执行用于对结果进行修正。可以是多个，按给定的顺序执行。
- use_upstream_id：新生成的 `Instance` 的 ID 将使用上游 `Instance`的 ID。
- target：对目标实例的一些要求，下面会有具体解释。
- delay：本次任务需要延迟指定的秒数后执行。
- delay_on_para：延迟本次任务的执行，延迟的时间=上游`Instance.para`中指定的位置的时间（元组中的第二个值）+给定的延时时间（元组中的第一个值）
- id_bridge: 如果此关系的下游的下游ID需要置为此关系的上游的ID，请设置此属性为 `true`

### 触发转换的条件： FlowSelector

```rust
pub struct FlowSelector {
    pub state_all: HashSet<String>,
    pub state_any: HashSet<String>,
    pub state_none: HashSet<String>,
    pub context_all: HashSet<String>,
    pub context_any: HashSet<String>,
    pub context_none: HashSet<String>,
    pub sys_context_all: HashSet<String>,
    pub sys_context_any: HashSet<String>,
    pub sys_context_none: HashSet<String>,
}

```


all of above are `and` relation

- state_[...]：上游 `Instance` 的状态必须满足[]中的要求。
- context_[...]：上游 `Instance` 的上下文必须满足[]中的要求。
- sys_context_[...]：上游`Instance`的系统上下文必须满足[]中的要求。

优先级

```
/// none: means can't include any one
/// all : means must include all
/// any : means must include one
```

**注意**：

尽管`上下文`和`系统上下文`都是 KV 类型，但当做流程选择条件时，Nature 只处理“K”不处理“V”，这是从简化设计角度来考虑的。V的形式是业务决定的，可能是一个URL，也可能“a|b|c”，也可能是个json，所以是不规范的。Nature 也不想对此进行规范，这样可能既限制了业务的灵活性又降低了处理性能。而“K”则是非常规范的，就是一个标签，非常便于 Nature 进行处理。当然这种方式也有问题，当`上下文`和`系统上下文`用作流程选择时就失去了KV的意义。

所以在一开始使用`上下文`和`系统上下文`时可能会出现错误的使用方式，如根据性别选择不同的处理流程：

- 错误的方式：K:gender,  V: boy | girl

  | KEY    | VALUE           |
  | ------ | --------------- |
  | gender | "boy" \| "girl" |

- 正确方式1：

  | KEY                       | VALUE |
  | ------------------------- | ----- |
  | gender.boy \| gender.girl | ""    |

  流程控制设置类似于：

  - 男孩流程：relation1.selector.**context_all** = ["gender.boy"]

  - 女孩流程：relation2.selector.**context_all** = ["gender.girl"]

- 正确方式2：

  | KEY          | VALUE |
  | ------------ | ----- |
  | gender.isBoy | ""    |
  
  流程控制设置类似于：
  
  - 男孩流程：relation1.selector.**context_all** = ["gender.isBoy"]
  
  - 女孩流程：relation2.selector.**context_none** = ["gender.isBoy"]

### Executor

Executor 目前有三种形态：转换器、前置过滤器、后置过滤器。其配置都采用下面的形式。

```rust
pub struct Executor {
    pub protocol: Protocol,
    pub url: String,
    pub settings: String,
}
```

**protocol**： Nature 与 `Executor`间的通讯协议，其内容不区分大小写，目前支持下面的方式。

- Http | Https：远程调用一个`Executor`。
- LocalRust：Nature 会加载一个本地 rust 库作为`Executor`
- Auto：当使用者不指定`executor`时，Nature在`运行时`会自动构建一个`executor`。因为`auto-executor`不会生成`Instance.content`的内容。所以当我们不需要关心实例的内容而只关心ID，状态等时可以不指定`executor`。
- BuiltIn：使用Nature 内置的转换器进行转换。通过 `url` 属性来指定要使用哪一个`builtin-executor`

**url**：用于定位`Executor`的位置

**settings**:`Executor`专有的配置，由具体的`Executor`给出。**注意** settings 的内容可以在运行时被 `Instance.sys_context`的`para.dynamic` 属性中的内容替换掉，而这种替换只局限于当前 Instance，不会影响后续 Instance 的替换。举例： 假设一个用于批量加载  Instance 的 beforter_filter 的 settings 配置如下：

```json
{
    "key_gt":"B:sale/item/(item_id):1|",
    "key_lt":"B:sale/item/(item_id):2|"
}
```

我们希望(item_id)在运行时被真正的ID 所替换。此时如果上游 instance.sys_context的 para.dynamic 属性中含有下面的定义，我们的愿望就可以实现了：

```properties
para.dynamic = "[[\"(item_id)\":\"123\"]]"
```

则上面的(item_id)会被替换为123。**注意**：目前 `para.dynamic` 只支持简单的替换，建议添加明确的边界符，如本示例用"()"，以避免发生错误的替换。

**`Executor`的示例**

```json
{
    "protocol":"Http",
    "url":"http://some_domain:8081/some_converter"
}
```

```json
{
    "protocol":"LocalRust",
    "url":"some_lib:some_converter"
}
```

```json
{
    "protocol":"builtIn",
    "url":"sum"
}
```

### RelationTarget

```
pub struct RelationTarget {
    pub states: Option<TargetState>,
    pub append_para: Vec<u8>,
    pub context_name: String,
}
```

**target_states**：可以增加或删除转化后的 `Instance` 的状态，状态必须在 `Meta` 中定义过。

**append_para**：该属性可指导 Nature 如何生成目标实例的 `para` 属性。示例，如其值为[3,1]， 假设上游para为 “a/b/c/d”，则目标实例的 `para` 值为 “d/b”。如果自身 `para` 已经有值， 则在此值的后面附加。**注意**下游 `Meta` 如果是状态数据则自身 **para**  不能有值，否则无法形成版本数据。

**context_name**: 这个只有在设置了 `append_para` 后才有效，Nature 会把 `append_para` 对应的值用作后续 `relation`配置中的参数替换，而 `context_name` 则指明了要替换的那个参数的名字。请见上方的 `Executor.settings` 说明

### 对目标状态的处理及要求：TargetState

```rust
pub struct TargetState {
    pub add: Option<Vec<String>>,		// 在上个状态基础上增加新的状态
    pub remove: Option<Vec<String>>,	// 从上个状态中删除指定的状态
    pub need_all: HashSet<String>,		// 上个目标状态必须拥有指定的状态
    pub need_any: HashSet<String>,		// 上个目标状态必须有一个或多个指定的状态
    pub need_none: HashSet<String>,		// 上个目标状态中不能含有任何一个指定的状态
}
```

## Executor接口形式

### Executor：转换器接口形式

`Executor` 用于实现 `Meta` 间 `Instance` 的转换，一般需要自己实现，Nature 也有内建及自动化的 `Executor` 实现。实现方式请参考[示例及功能讲解](https://github.com/llxxbb/Nature-Demo)。

`Executor`只有一个入参和一个出参。

**入参：ConverterParameter**

```rust
pub struct ConverterParameter {
    pub from: Instance,					// 上游数据实例
    pub last_state: Option<Instance>,	// 最近一次状态目标的数据实例
    pub task_id: Vec<u8>,				// 此次任务ID，延时处理时回调Nature的凭据。
    pub master: Option<Instance>,		// 上游 mater的数据实例（ID相同）
    pub cfg: String,					// json 对象，`Executor`自有的配置。
}
```

**出参：**

```rust
pub enum ConverterReturned {
    LogicalError(String),				// 逻辑错误，Nature 不会重试
    EnvError(String),					// 当前条件不满足，Nature 会在将来的某个时刻重试
    None,								// 没有数据返回
    Delay(u32),							// 用于延时处理，具体用法请看Demo
    Instances(Vec<Instance>),			// 产出的目标数据实例
    SelfRoute(Vec<SelfRouteInstance>),	// 定义动态路由
}
```

### Executor：filter_before 接口形式

filter_before 需要使用者自行实现,下面为LocalRust的实现形式

```rust
#[no_mangle]
#[allow(improper_ctypes)]
pub extern fn your_func(para: &Instance) -> Result<Instance> {
	// TODO your logic
}
```

### Executor：filter_after 接口形式

filter_after 需要使用者自行实现,下面为LocalRust的实现形式

```rust
#[no_mangle]
#[allow(improper_ctypes)]
pub extern fn your_func(para: &Vec<Instance>) -> Result<Vec<Instance>> {
	// TODO your logic
}
```

## 动态`Executor`转换器

动态路由不需要在运行之前预先定义，既在运行时决定自己的去处，非常的灵活，每个实例可以有自己独立的选择。不过不建议使用，一是目前此功能还不完善，二是该功能性能比静态路由要差，三、业务布局的展示会比较困难。

