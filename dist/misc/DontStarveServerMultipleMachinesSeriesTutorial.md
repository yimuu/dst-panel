# 饥荒服务器多台机器串联教程-如此简单

> 以下教程是通用的

## 介绍

当我们云服务器性能不行，不足以开两个世界及其以上时，我们可以把压力分摊到多台服务器上，从服务器可以是本地也可以是云服。

如图所示：

![Untitled](misc/images/DontStarveServerMultipleMachinesSeriesTutorial/Untitled.png)

我们可以开三层世界，其中只需要主世界在公网，其他从世界可以不用在公网内

> 💡注意：主世界的通信端口 **master_port** 需要开放udp端口

## 示例

假设我们需要串联一台机器

现在有主世界 **139.159.184.218**，从世界1 为本地开服


| 服务器  | ip              | 身份         | 公网ip |
| ------- | --------------- | ------------ | ------ |
| 主世界  | 139.159.184.218 | 主世界(地面) | 需要   |
| 从世界1 | 192.168.9.203   | 从世界(洞穴) | 不需要 |

>💡注意这里只是演示两层，如果是三层及其以上需要添加模组 《多层选择器》
> **同时所有的世界的token令牌需要保持一致**

### 主世界

我们打开云服务的面板（**主世界**），打开房间设置，点击 `全部`  ，在多世界设置里面。

- **绑定ip: 0.0.0.0**
- **主世界ip: 139.159.184.218**

如果需要加密，则把通信密码也写上，需要保持一致

![Untitled](misc/images/DontStarveServerMultipleMachinesSeriesTutorial/Untitled%201.png)

### 从世界

回到本地面板，我们也只需要修改 绑定ip 和 主世界ip

- **绑定ip: 127.0.0.1**
- **主世界ip: 139.159.184.218**

如果需要加密，则把通信密码也写上，需要保持一致

![Untitled](misc/images/DontStarveServerMultipleMachinesSeriesTutorial/Untitled%202.png)

### 启动

依次启动 **主世界—> 从世界1—>…**

启动主世界，主世界日志

```go
[00:00:37]: Validating portal[4] <-> <nil>[4] (inactive)
[00:00:37]: Validating portal[5] <-> <nil>[5] (inactive)
[00:00:37]: Validating portal[6] <-> <nil>[6] (inactive)
[00:00:37]: Validating portal[7] <-> <nil>[7] (inactive)
[00:00:37]: Validating portal[8] <-> <nil>[8] (inactive)
[00:00:37]: Validating portal[9] <-> <nil>[9] (inactive)
[00:00:37]: Validating portal[10] <-> <nil>[10] (inactive)
[00:00:37]: Server registered via geo DNS in ap-southeast-1
[00:00:37]: Sim paused
```

启动从世界1

```go
[00:00:46]: World 1 is now connected
[00:00:46]: Telling Client our new session identifier: 30A50CF48D381EF6

[00:00:46]: [SyncWorldSettings] recieved world settings from master shard.	true
[00:00:46]: [SyncWorldSettings] applying hunger = default from master shard.
[00:00:46]: [SyncWorldSettings] applying basicresource_regrowth = none from master shard.
[00:00:46]: Validating portal[10] <-> 1[10] (active)
[00:00:46]: Validating portal[1] <-> 1[1] (active)
[00:00:46]: Validating portal[9] <-> 1[9] (active)
[00:00:46]: Validating portal[8] <-> 1[8] (active)
[00:00:46]: Validating portal[2] <-> 1[2] (active)
[00:00:46]: Validating portal[5] <-> 1[5] (active)
[00:00:46]: Validating portal[4] <-> 1[4] (active)
[00:00:46]: Validating portal[3] <-> 1[3] (active)
[00:00:46]: Validating portal[6] <-> 1[6] (active)
[00:00:46]: Validating portal[7] <-> 1[7] (active)
[00:00:47]: [Shard] secondary shard LUA is now ready!
[00:00:47]: Sim paused
```

**[00:00:47]: [Shard] secondary shard LUA is now ready! 代表显示连接主世界成功了**

现在就完成了，我们打开查服，查看当前房间名称，点击层数，看世界是否已经连接起来了

![Untitled](misc/images/DontStarveServerMultipleMachinesSeriesTutorial/Untitled%203.png)

现在让我们一起开始把服务器都串起来玩吧，只需要有一台云服，其他都可以是本地电脑

## 注意项

1. 所有世界模组，像任务模组，五格这种类似尽量保持模组一致，如果不一致可能导致跳世界会丢失或者报错等
2. 所有世界启动时用的令牌必须要一样，就是 `cluster_token.txt` 文件里的内容要一样
3. 如果游戏本体更新，则所有世界的游戏本体都需要更新
4. 增减模组也是需要所有世界都要统一修改并重启服务
5. 从世界里不能开天体
6. 从世界里不能换人

## 参考

1. [https://atjiu.github.io/dstmod-tutorial/#/multi_dedicated_server](https://atjiu.github.io/dstmod-tutorial/#/multi_dedicated_server)